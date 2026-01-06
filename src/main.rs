mod auth;
mod config;
mod terminal;

use axum::{
    middleware,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("\nRequired environment variables:");
            eprintln!("  TERM_USERNAME - Basic auth username");
            eprintln!("  TERM_PASSWORD - Basic auth password");
            eprintln!("\nOptional:");
            eprintln!("  TERM_PORT     - Server port (default: 3000)");
            eprintln!("  TERM_SHELL    - Shell to spawn (default: /bin/sh)");
            eprintln!("  TERM_SESSION  - tmux session name (default: main)");
            std::process::exit(1);
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    // Routes that require authentication
    let protected_routes = Router::new()
        .nest_service("/", ServeDir::new("static"))
        .layer(middleware::from_fn_with_state(config.clone(), auth::basic_auth));

    // WebSocket route (auth handled in handler via query param)
    let ws_route = Router::new()
        .route("/ws", get(terminal::ws_handler))
        .with_state(config.clone());

    let app = Router::new()
        .merge(ws_route)
        .merge(protected_routes);

    info!("Starting server on http://{}", addr);
    info!("tmux session: {} (attach with: tmux attach -t {})", config.session, config.session);
    info!("Default shell: {}", config.shell);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
