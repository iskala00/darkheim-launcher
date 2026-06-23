use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;

use crate::progress::ProgressCounters;

pub fn java_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkheim/java/17")
}

pub fn java_bin_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        java_dir().join("bin/java.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        java_dir().join("bin/java")
    }
}

pub fn find_java_executable(dir: &PathBuf) -> Option<PathBuf> {
    let mut queue = vec![dir.clone()];
    while let Some(current) = queue.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    queue.push(path);
                } else if is_java_executable_name(
                    path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                ) {
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

#[cfg(target_os = "windows")]
fn is_java_executable_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("java.exe")
}

#[cfg(not(target_os = "windows"))]
fn is_java_executable_name(name: &str) -> bool {
    name == "java"
}

pub async fn download_java(counters: Arc<ProgressCounters>) -> Result<PathBuf, String> {
    let os = if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err("Unsupported operating system for Java download".into());
    };

    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return Err("Unsupported architecture for Java download".into());
    };

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/17/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
        os, arch
    );

    let dir = java_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let archive_path = dir.join(format!("java.{}", ext));

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Java download failed: {}", e))?;
    let total_size = response.content_length().unwrap_or(0);
    if total_size > 0 {
        counters.add_total(total_size);
    }

    let mut response = response;
    let mut file = tokio::fs::File::create(&archive_path)
        .await
        .map_err(|e| format!("Failed to create Java archive: {}", e))?;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write Java archive: {}", e))?;
        counters.add_downloaded(chunk.len() as u64);
    }
    file.flush()
        .await
        .map_err(|e| format!("Failed to flush Java archive: {}", e))?;

    Ok(archive_path)
}

pub async fn extract_java(archive_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let archive_path = archive_path.clone();
    let dest = dest.clone();
    tokio::task::spawn_blocking(move || {
        if archive_path.to_string_lossy().ends_with(".zip") {
            extract_zip(&archive_path, &dest)
        } else {
            extract_tar_gz(&archive_path, &dest)
        }
    })
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

fn extract_zip(zip_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(dest).map_err(|e| e.to_string())?;
    Ok(())
}
