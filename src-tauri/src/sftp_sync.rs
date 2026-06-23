use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use russh::client;
use russh_sftp::client::{Config as SftpConfig, SftpSession};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

use crate::env::SftpCredentials;
use crate::progress::ProgressCounters;

pub struct SyncResult {
    pub files_to_download: usize,
}

pub async fn sync(
    credentials: &SftpCredentials,
    instance_dir: &Path,
    folders: &[&str],
    counters: Arc<ProgressCounters>,
) -> Result<SyncResult, String> {
    let (session, sftp) = connect(credentials).await?;

    let mut remote_files = Vec::new();
    for folder in folders {
        let remote_path = folder.to_string();
        let exists = sftp
            .try_exists(&remote_path)
            .await
            .map_err(|e| format!("Failed to check remote folder {}: {}", remote_path, e))?;
        if !exists {
            continue;
        }
        list_remote_files(&sftp, &remote_path, folder, &mut remote_files).await?;
    }

    let mut files_to_download: Vec<RemoteFile> = Vec::new();
    let remote_set: HashSet<String> = remote_files
        .iter()
        .map(|f| f.relative_path.clone())
        .collect();

    for file in remote_files {
        let local_path = instance_dir.join(&file.relative_path);
        let needs_download = if local_path.exists() {
            let local_meta = tokio::fs::metadata(&local_path)
                .await
                .map_err(|e| e.to_string())?;
            local_meta.len() != file.size
        } else {
            true
        };

        if needs_download {
            files_to_download.push(file);
        }
    }

    // Delete local files that are not present on the server.
    let mut local_files = Vec::new();
    for folder in folders {
        let local_folder = instance_dir.join(folder);
        if local_folder.exists() {
            collect_local_files(&local_folder, folder, &mut local_files).await?;
        }
    }
    for local_file in local_files {
        if !remote_set.contains(&local_file) {
            let path = instance_dir.join(&local_file);
            tokio::fs::remove_file(&path).await.ok();
        }
    }

    let total_bytes: u64 = files_to_download.iter().map(|f| f.size).sum();
    counters.add_total(total_bytes);

    let result = SyncResult {
        files_to_download: files_to_download.len(),
    };

    let _ = sftp.close().await;
    let _ = session
        .disconnect(russh::Disconnect::ByApplication, "done", "EN")
        .await;

    if !files_to_download.is_empty() {
        const CONCURRENCY: usize = 8;
        let worker_count = CONCURRENCY.min(files_to_download.len());
        let sessions = connect_pool(credentials, worker_count).await?;
        let queue = Arc::new(Mutex::new(VecDeque::from(files_to_download)));
        let instance_dir = instance_dir.to_path_buf();
        let mut set = tokio::task::JoinSet::new();
        for (session, sftp) in sessions {
            let q = queue.clone();
            let counters = counters.clone();
            let instance_dir = instance_dir.clone();
            set.spawn(async move {
                loop {
                    let file = {
                        let mut q = q.lock().unwrap();
                        q.pop_front()
                    };
                    match file {
                        Some(file) => {
                            let remote_path = file.relative_path;
                            let local_path = instance_dir.join(&remote_path);
                            download_file(&sftp, &remote_path, &local_path, &counters).await?;
                        }
                        None => break,
                    }
                }
                let _ = sftp.close().await;
                let _ = session
                    .disconnect(russh::Disconnect::ByApplication, "done", "EN")
                    .await;
                Ok::<(), String>(())
            });
        }
        while let Some(res) = set.join_next().await {
            res.map_err(|e| format!("Download task panicked: {}", e))??;
        }
    }

    Ok(result)
}

#[derive(Clone)]
struct RemoteFile {
    relative_path: String,
    size: u64,
}

struct SshClient;

impl client::Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

async fn connect(
    credentials: &SftpCredentials,
) -> Result<(client::Handle<SshClient>, SftpSession), String> {
    timeout(Duration::from_secs(20), connect_inner(credentials))
        .await
        .map_err(|_| "SFTP connection timed out".to_string())?
}

async fn connect_inner(
    credentials: &SftpCredentials,
) -> Result<(client::Handle<SshClient>, SftpSession), String> {
    let config = client::Config::default();
    let handler = SshClient;
    let mut session = client::connect(
        Arc::new(config),
        (credentials.host.as_str(), credentials.port),
        handler,
    )
    .await
    .map_err(|e| format!("SFTP connection failed: {}", e))?;

    let auth_res = session
        .authenticate_password(credentials.user.clone(), credentials.password.clone())
        .await
        .map_err(|e| format!("SFTP authentication failed: {}", e))?;

    if !auth_res.success() {
        return Err("SFTP authentication failed: invalid credentials".into());
    }

    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| format!("SFTP channel failed: {}", e))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("SFTP subsystem failed: {}", e))?;

    let sftp_config = SftpConfig {
        request_timeout_secs: 120,
        max_packet_len: 1024 * 1024,
        ..Default::default()
    };
    let sftp = SftpSession::new_with_config(channel.into_stream(), sftp_config)
        .await
        .map_err(|e| format!("SFTP session failed: {}", e))?;

    Ok((session, sftp))
}

async fn connect_pool(
    credentials: &SftpCredentials,
    count: usize,
) -> Result<Vec<(client::Handle<SshClient>, SftpSession)>, String> {
    let mut results = Vec::with_capacity(count);
    for _ in 0..count {
        results.push(connect(credentials).await?);
    }
    Ok(results)
}

async fn list_remote_files(
    sftp: &SftpSession,
    remote_path: &str,
    relative_prefix: &str,
    out: &mut Vec<RemoteFile>,
) -> Result<(), String> {
    let mut stack = vec![(remote_path.to_string(), relative_prefix.to_string())];

    while let Some((remote_path, relative_prefix)) = stack.pop() {
        let mut entries = sftp
            .read_dir(&remote_path)
            .await
            .map_err(|e| format!("Failed to read remote dir {}: {}", remote_path, e))?;

        while let Some(entry) = entries.next() {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }

            let full_remote = format!("{}/{}", remote_path, name);
            let relative = if relative_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_prefix, name)
            };

            let file_type = entry.file_type();
            if file_type.is_dir() {
                stack.push((full_remote, relative));
            } else if file_type.is_file() {
                let meta = entry.metadata();
                out.push(RemoteFile {
                    relative_path: relative,
                    size: meta.len(),
                });
            }
        }
    }

    Ok(())
}

async fn collect_local_files(
    dir: &Path,
    relative_prefix: &str,
    out: &mut Vec<String>,
) -> Result<(), String> {
    let mut stack = vec![(dir.to_path_buf(), relative_prefix.to_string())];

    while let Some((dir, relative_prefix)) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .map_err(|e| format!("Failed to read local dir {}: {}", dir.display(), e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = if relative_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_prefix, name)
            };

            if path.is_dir() {
                stack.push((path, relative));
            } else {
                out.push(relative);
            }
        }
    }

    Ok(())
}

async fn download_file(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &PathBuf,
    counters: &Arc<ProgressCounters>,
) -> Result<(), String> {
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
    }

    let tmp_path = local_path.with_extension("tmp");
    let mut remote_file = sftp
        .open(remote_path)
        .await
        .map_err(|e| format!("Failed to open remote file {}: {}", remote_path, e))?;
    let mut local_file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("Failed to create local file {}: {}", tmp_path.display(), e))?;

    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let n = remote_file
            .read(&mut buffer[..])
            .await
            .map_err(|e| format!("Failed to read remote file {}: {}", remote_path, e))?;
        if n == 0 {
            break;
        }
        local_file
            .write_all(&buffer[..n])
            .await
            .map_err(|e| format!("Failed to write local file {}: {}", tmp_path.display(), e))?;
        counters.add_downloaded(n as u64);
    }

    local_file
        .flush()
        .await
        .map_err(|e| format!("Failed to flush local file {}: {}", tmp_path.display(), e))?;
    drop(local_file);
    drop(remote_file);

    tokio::fs::rename(&tmp_path, local_path)
        .await
        .map_err(|e| {
            format!(
                "Failed to rename {} to {}: {}",
                tmp_path.display(),
                local_path.display(),
                e
            )
        })?;

    Ok(())
}
