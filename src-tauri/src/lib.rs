mod env;
mod java;
mod launcher;
mod progress;
mod server;
mod sftp_sync;

#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub phase: String,
    pub message: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f64,
}

#[tauri::command]
async fn start_game(window: tauri::Window, nickname: String) -> Result<(), String> {
    let nickname = nickname.trim().to_string();
    if nickname.is_empty() {
        return Err("Введите ник".into());
    }
    launcher::launch(&window, &nickname)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_data() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let new_base = home.join(".darkheim");
    let old_base = home.join("Library/Application Support/Darkheim");
    if new_base.exists() {
        tokio::fs::remove_dir_all(&new_base)
            .await
            .map_err(|e| format!("Failed to remove {}: {}", new_base.display(), e))?;
    }
    if old_base.exists() {
        tokio::fs::remove_dir_all(&old_base)
            .await
            .map_err(|e| format!("Failed to remove {}: {}", old_base.display(), e))?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenvy::from_path("../.env").or_else(|_| dotenvy::from_path(".env"));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_game, clear_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
