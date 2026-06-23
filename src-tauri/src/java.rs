use std::path::PathBuf;
use std::sync::Arc;

use crate::progress::ProgressCounters;

pub fn java_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkheim/java/17")
}

pub fn java_bin_path() -> PathBuf {
    java_dir().join("bin/java")
}

pub fn find_java_executable(dir: &PathBuf) -> Option<PathBuf> {
    let mut queue = vec![dir.clone()];
    while let Some(current) = queue.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    queue.push(path);
                } else if path.file_name().and_then(|n| n.to_str()) == Some("java") {
                    if path
                        .parent()
                        .and_then(|p| p.file_name().and_then(|n| n.to_str()))
                        == Some("bin")
                    {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

pub async fn download_java(counters: Arc<ProgressCounters>) -> Result<PathBuf, String> {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return Err("Unsupported architecture for Java download".into());
    };

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/17/ga/mac/{}/jre/hotspot/normal/eclipse?project=jdk",
        arch
    );

    let dir = java_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let tar_path = dir.join("java.tar.gz");

    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let total_size = response.content_length().unwrap_or(0);
    if total_size > 0 {
        counters.add_total(total_size);
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    counters.add_downloaded(bytes.len() as u64);
    tokio::fs::write(&tar_path, bytes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tar_path)
}

pub async fn extract_java(tar_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let tar_path = tar_path.clone();
    let dest = dest.clone();
    tokio::task::spawn_blocking(move || extract_tar_gz(&tar_path, &dest))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn extract_tar_gz(tar_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(tar_path).map_err(|e| e.to_string())?;
    let tar = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(dest).map_err(|e| e.to_string())?;
    Ok(())
}
