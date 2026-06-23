use std::env;

pub struct SftpCredentials {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

pub fn load_sftp_credentials() -> Result<SftpCredentials, String> {
    // Try loading .env from the project root (dev) or src-tauri directory.
    let _ = dotenvy::from_path("../.env").or_else(|_| dotenvy::from_path(".env"));

    let host = option_env!("SFTP_HOST")
        .map(|v| v.to_string())
        .or_else(|| env::var("SFTP_HOST").ok())
        .ok_or("SFTP_HOST not set")?;
    let user = option_env!("SFTP_USER")
        .map(|v| v.to_string())
        .or_else(|| env::var("SFTP_USER").ok())
        .ok_or("SFTP_USER not set")?;
    let password = option_env!("SFTP_PASSWORD")
        .map(|v| v.to_string())
        .or_else(|| env::var("SFTP_PASSWORD").ok())
        .ok_or("SFTP_PASSWORD not set")?;

    let (host, port) = parse_sftp_host(&host)?;
    Ok(SftpCredentials {
        host,
        port,
        user,
        password,
    })
}

fn parse_sftp_host(s: &str) -> Result<(String, u16), String> {
    let s = s.trim();
    let without_scheme = s
        .strip_prefix("sftp://")
        .or_else(|| s.strip_prefix("ssh://"))
        .or_else(|| s.strip_prefix("ftp://"))
        .unwrap_or(s);

    if let Some((host, port_str)) = without_scheme.rsplit_once(':') {
        let port = port_str
            .parse::<u16>()
            .map_err(|_| format!("Invalid SFTP port: {}", port_str))?;
        Ok((host.to_string(), port))
    } else {
        Ok((without_scheme.to_string(), 22))
    }
}
