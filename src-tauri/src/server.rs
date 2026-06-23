// Hardcoded Minecraft server address.
// This whole file is intended to be replaced later by a backend request.

const MINECRAFT_SERVER: &str = "37.9.13.3:32991";

pub fn minecraft_server() -> (String, u16) {
    let mut parts = MINECRAFT_SERVER.split(':');
    let host = parts.next().unwrap_or("37.9.13.3").to_string();
    let port = parts.next().and_then(|p| p.parse().ok()).unwrap_or(25565);
    (host, port)
}
