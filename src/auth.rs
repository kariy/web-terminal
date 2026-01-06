use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::config::Config;

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic realm=\"Web Terminal\"")],
        "Unauthorized",
    )
        .into_response()
}

pub async fn basic_auth(
    State(config): State<Config>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Basic ") => {
            let credentials = auth.trim_start_matches("Basic ");
            match STANDARD.decode(credentials) {
                Ok(decoded) => {
                    let credential_str = String::from_utf8_lossy(&decoded);
                    let mut parts = credential_str.splitn(2, ':');
                    let username = parts.next().unwrap_or("");
                    let password = parts.next().unwrap_or("");

                    if username == config.username && password == config.password {
                        next.run(request).await
                    } else {
                        unauthorized()
                    }
                }
                Err(_) => unauthorized(),
            }
        }
        _ => unauthorized(),
    }
}

/// Extracts credentials from a WebSocket URL query parameter
/// Format: ?auth=base64(username:password)
pub fn validate_ws_auth(query: Option<&str>, config: &Config) -> bool {
    let query = match query {
        Some(q) => q,
        None => return false,
    };

    // Parse query string to find auth parameter
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("auth=") {
            if let Ok(decoded) = STANDARD.decode(value) {
                let credential_str = String::from_utf8_lossy(&decoded);
                let mut parts = credential_str.splitn(2, ':');
                let username = parts.next().unwrap_or("");
                let password = parts.next().unwrap_or("");
                return username == config.username && password == config.password;
            }
        }
    }
    false
}
