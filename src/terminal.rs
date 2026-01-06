use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{collections::HashMap, io::{Read, Write}, thread};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::{auth::validate_ws_auth, config::Config};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(config): State<Config>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    // Validate auth from query parameter
    let query_str = query
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    if !validate_ws_auth(Some(&query_str), &config) {
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, config))
}

async fn handle_socket(socket: WebSocket, config: Config) {
    info!("New WebSocket connection");

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create PTY
    let pty_system = native_pty_system();
    let pty_pair = match pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            error!("Failed to open PTY: {}", e);
            let _ = ws_sender.send(Message::Text(format!("Error: {}", e))).await;
            return;
        }
    };

    // Spawn tmux session (creates new or attaches to existing)
    // This allows sharing the terminal between web and direct server access
    // We use a shell command to:
    // 1. Create a detached session if it doesn't exist
    // 2. Attach to the session
    // This ensures the session persists even when all clients disconnect
    let mut cmd = CommandBuilder::new("sh");
    let tmux_cmd = format!(
        "tmux has-session -t {0} 2>/dev/null || tmux new-session -d -s {0}; exec tmux attach -t {0}",
        &config.session
    );
    cmd.args(["-c", &tmux_cmd]);
    cmd.env("TERM", "xterm-256color");
    // Set default shell for new tmux sessions
    cmd.env("SHELL", &config.shell);

    let child = match pty_pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn tmux: {}", e);
            let _ = ws_sender
                .send(Message::Text(format!("Error spawning tmux: {}", e)))
                .await;
            return;
        }
    };

    // Get master PTY for reading/writing
    let master = pty_pair.master;
    let mut reader = master.try_clone_reader().unwrap();
    let mut writer = master.take_writer().unwrap();

    // Channel for PTY output -> WebSocket
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

    // Spawn thread to read from PTY
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Task to forward PTY output to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if ws_sender.send(Message::Binary(data)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming WebSocket messages (master is still available for resize)
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if writer.write_all(&data).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
            Ok(Message::Text(text)) => {
                // Handle resize messages: "resize:cols:rows"
                if text.starts_with("resize:") {
                    let parts: Vec<&str> = text.split(':').collect();
                    if parts.len() == 3 {
                        if let (Ok(cols), Ok(rows)) = (parts[1].parse(), parts[2].parse()) {
                            let _ = master.resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                    }
                } else {
                    // Regular text input
                    if writer.write_all(text.as_bytes()).is_err() {
                        break;
                    }
                    let _ = writer.flush();
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // Cleanup - don't kill the tmux session, just abort the send task
    // The tmux client will detach when the PTY closes
    drop(child);
    send_task.abort();
    info!("WebSocket connection closed (tmux session '{}' persists)", config.session);
}
