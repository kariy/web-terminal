use std::env;

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub username: String,
    pub password: String,
    pub shell: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let username = env::var("TERM_USERNAME")
            .map_err(|_| "TERM_USERNAME environment variable is required")?;
        let password = env::var("TERM_PASSWORD")
            .map_err(|_| "TERM_PASSWORD environment variable is required")?;

        let port = env::var("TERM_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| "TERM_PORT must be a valid port number")?;

        let shell = env::var("TERM_SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

        Ok(Config {
            port,
            username,
            password,
            shell,
        })
    }
}
