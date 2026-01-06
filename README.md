# Web Terminal

A web-based terminal emulator that exposes a server's terminal session in the browser. Built with Rust (axum) and TypeScript (xterm.js).

## Features

- Interactive terminal in the browser
- Shared sessions via tmux - access the same terminal from web AND server simultaneously
- Basic authentication
- Terminal resize support

## Building

### Prerequisites

- Rust (1.70+)
- Node.js (18+)
- tmux installed on the server

### Build

```bash
# Build frontend
cd web && npm install && npm run build && cd ..

# Build backend
cargo build --release
```

## Running

```bash
# Required
export TERM_USERNAME=your_username
export TERM_PASSWORD=your_password

# Optional
export TERM_PORT=3000        # default: 3000
export TERM_SHELL=/bin/bash  # default: /bin/sh
export TERM_SESSION=main     # default: main

# Run
./target/release/web-terminal
```

Then open `http://localhost:3000` in your browser.

## Shared Sessions

The terminal uses tmux under the hood, which means you can access the same session from multiple places:

### From the web
Just open `http://localhost:3000` and log in.

### From the server (SSH)
```bash
tmux attach -t main
```

Both the web client and server terminal will see the same session. You can type in either one and both will update in real-time.

### How it works

```
┌──────────────┐
│  Browser     │───┐
│  (web)       │   │
└──────────────┘   │      ┌─────────────┐      ┌─────────────┐
                   ├─────►│ tmux session │─────►│    shell    │
┌──────────────┐   │      │   "main"     │      │  (bash/sh)  │
│  Server      │───┘      └─────────────┘      └─────────────┘
│  (terminal)  │
└──────────────┘
```

When you connect via web, the server attaches to (or creates) the tmux session. When you SSH into the server and run `tmux attach`, you're connecting to the same session.

## Basic tmux Commands

Once attached to a tmux session, these commands might be helpful:

| Command | Description |
|---------|-------------|
| `tmux attach -t main` | Attach to session named "main" |
| `tmux detach` or `Ctrl+b d` | Detach from session (keeps it running) |
| `tmux ls` | List all sessions |
| `tmux kill-session -t main` | Kill the session named "main" |

### Inside tmux

| Shortcut | Description |
|----------|-------------|
| `Ctrl+b d` | Detach from session |
| `Ctrl+b c` | Create new window |
| `Ctrl+b n` | Next window |
| `Ctrl+b p` | Previous window |
| `Ctrl+b [` | Enter scroll mode (use arrows, `q` to exit) |
| `Ctrl+b %` | Split pane vertically |
| `Ctrl+b "` | Split pane horizontally |
| `Ctrl+b arrow` | Move between panes |

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `TERM_USERNAME` | Basic auth username | (required) |
| `TERM_PASSWORD` | Basic auth password | (required) |
| `TERM_PORT` | Server port | 3000 |
| `TERM_SHELL` | Shell for new sessions | /bin/sh |
| `TERM_SESSION` | tmux session name | main |

## Security Notes

- Use HTTPS in production (put behind a reverse proxy like nginx/caddy)
- Choose strong credentials
- Consider firewall rules to restrict access
