use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{Emitter, Window};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::env;
use crate::java;
use crate::progress::ProgressCounters;
use crate::server;
use crate::sftp_sync;
use crate::ProgressPayload;

const MANAGED_FOLDERS: &[&str] = &["mods", "config", "resourcepacks"];

pub async fn launch(window: &Window, nickname: &str) -> Result<(), String> {
    let phase = Arc::new(Mutex::new("checking".to_string()));
    let counters = ProgressCounters::new();
    let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();
    let progress_handle =
        spawn_progress_task(window.clone(), counters.clone(), phase.clone(), stop_rx);

    let result = launch_inner(window, nickname, phase.clone(), counters.clone()).await;

    let _ = stop_tx.send(());
    progress_handle.await.ok();

    result
}

fn spawn_progress_task(
    window: Window,
    counters: Arc<ProgressCounters>,
    phase: Arc<Mutex<String>>,
    mut stop: mpsc::UnboundedReceiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(200));
        loop {
            tokio::select! {
                _ = stop.recv() => break,
                _ = tick.tick() => {
                    let (downloaded, total) = counters.get();
                    let progress = if total > 0 {
                        downloaded as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    let phase_str = phase.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let _ = window.emit("launcher:progress", ProgressPayload {
                        phase: phase_str,
                        message: String::new(),
                        downloaded_bytes: downloaded,
                        total_bytes: total,
                        progress,
                    });
                }
            }
        }
    })
}

async fn launch_inner(
    _window: &Window,
    nickname: &str,
    phase: Arc<Mutex<String>>,
    counters: Arc<ProgressCounters>,
) -> Result<(), String> {
    migrate_old_data().await?;
    let instance_dir = instance_dir()?;
    let credentials = env::load_sftp_credentials()?;
    let (server_host, server_port) = server::minecraft_server();

    let client_installed = instance_dir.join("versions").is_dir();

    set_phase(&phase, "checking");
    set_phase(&phase, "syncing");

    let sync_result = sftp_sync::sync(
        &credentials,
        &instance_dir,
        MANAGED_FOLDERS,
        counters.clone(),
    )
    .await?;

    if !client_installed {
        set_phase(&phase, "downloading_launcher");
    } else if sync_result.files_to_download > 0 {
        set_phase(&phase, "updating");
    }

    let java_path = if java::java_bin_path().exists() {
        java::java_bin_path()
    } else if let Some(found) = java::find_java_executable(&java::java_dir()) {
        found
    } else {
        set_phase(&phase, "downloading_java");
        let tar_path = java::download_java(counters.clone()).await?;
        set_phase(&phase, "installing");
        java::extract_java(&tar_path, &java::java_dir()).await?;
        tokio::fs::remove_file(&tar_path).await.ok();
        java::find_java_executable(&java::java_dir())
            .ok_or("Java executable not found after extraction")?
    };

    {
        let current_phase = phase.lock().unwrap_or_else(|e| e.into_inner());
        if current_phase.as_str() == "checking" {
            set_phase(&phase, "installing");
        }
    }
    let instance_dir_clone = instance_dir.clone();
    let nickname = nickname.to_string();
    let java_path_clone = java_path.clone();
    let server_host = server_host.clone();
    let (jvm_file, mc_dir, main_class, mut jvm_args, game_args) =
        tokio::task::spawn_blocking(move || {
            install_game(
                &instance_dir_clone,
                &nickname,
                &java_path_clone,
                &server_host,
                server_port,
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    set_phase(&phase, "launching");

    #[cfg(target_os = "macos")]
    if !jvm_args.iter().any(|a| a == "-XstartOnFirstThread") {
        jvm_args.insert(0, "-XstartOnFirstThread".to_string());
    }
    if !jvm_args.iter().any(|a| a.starts_with("-Xmx")) {
        jvm_args.push("-Xmx4G".to_string());
    }
    if !jvm_args.iter().any(|a| a.starts_with("-XX:ErrorFile=")) {
        jvm_args.push(format!(
            "-XX:ErrorFile={}",
            mc_dir.join("hs_err.log").display()
        ));
    }

    let log_dir = mc_dir.join("logs");
    tokio::fs::create_dir_all(&log_dir)
        .await
        .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    let log_path = log_dir.join("latest.log");
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Failed to create game log: {}", e))?;

    let mut child = std::process::Command::new(&jvm_file)
        .current_dir(&mc_dir)
        .env_remove("DYLD_LIBRARY_PATH")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("DYLD_FORCE_FLAT_NAMESPACE")
        .args(&jvm_args)
        .arg(&main_class)
        .args(&game_args)
        .stdout(Stdio::from(
            log_file
                .try_clone()
                .map_err(|e| format!("Failed to clone log file: {}", e))?,
        ))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| format!("Failed to start Minecraft: {}", e))?;

    tokio::time::sleep(Duration::from_secs(5)).await;
    if let Ok(Some(status)) = child.try_wait() {
        if !status.success() {
            let tail = std::fs::read_to_string(&log_path)
                .ok()
                .map(|s| {
                    s.lines()
                        .rev()
                        .take(200)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            let hs_err = latest_hs_err_path(&mc_dir).and_then(|p| {
                std::fs::read_to_string(&p).ok().map(|s| {
                    (
                        p,
                        s.lines()
                            .rev()
                            .take(100)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                })
            });
            let hs_err_section = hs_err
                .map(|(p, s)| format!("\n\nJVM crash log: {}\n{}", p.display(), s))
                .unwrap_or_default();
            return Err(format!(
                "Minecraft exited unexpectedly (code: {:?})\nFull log: {}\n{}{}",
                status.code(),
                log_path.display(),
                tail,
                hs_err_section
            ));
        }
    }

    Ok(())
}

fn set_phase(phase: &Arc<Mutex<String>>, value: &str) {
    if let Ok(mut p) = phase.lock() {
        *p = value.to_string();
    }
}

fn instance_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    Ok(home.join(".darkheim/instance"))
}

async fn migrate_old_data() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let old_base = home.join("Library/Application Support/Darkheim");
    let new_base = home.join(".darkheim");

    let old_instance = old_base.join("instances/default");
    let new_instance = new_base.join("instance");
    if old_instance.exists() && !new_instance.exists() {
        if let Some(parent) = new_instance.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::rename(&old_instance, &new_instance)
            .await
            .map_err(|e| format!("Failed to migrate instance dir: {}", e))?;
    }

    let old_java = old_base.join("java/17");
    let new_java = new_base.join("java/17");
    if old_java.exists() && !new_java.exists() {
        if let Some(parent) = new_java.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::rename(&old_java, &new_java)
            .await
            .map_err(|e| format!("Failed to migrate Java dir: {}", e))?;
    }

    Ok(())
}

fn install_game(
    instance_dir: &PathBuf,
    nickname: &str,
    java_path: &PathBuf,
    server_host: &str,
    server_port: u16,
) -> Result<(PathBuf, PathBuf, String, Vec<String>, Vec<String>), String> {
    let mut installer = portablemc::forge::Installer::new(
        portablemc::forge::Loader::Forge,
        portablemc::forge::Version::Stable("1.20.1".to_string()),
    );

    installer
        .mojang_mut()
        .base_mut()
        .set_main_dir(instance_dir.clone());

    installer
        .mojang_mut()
        .set_auth_offline_username(nickname)
        .set_quick_play(portablemc::moj::QuickPlay::Multiplayer {
            host: server_host.to_string(),
            port: server_port,
        });

    installer
        .mojang_mut()
        .base_mut()
        .set_jvm_policy(portablemc::base::JvmPolicy::Static(java_path.clone()));

    let game = installer.install(()).map_err(|e| e.to_string())?;

    Ok((
        game.jvm_file,
        game.mc_dir,
        game.main_class,
        game.jvm_args,
        game.game_args,
    ))
}

fn latest_hs_err_path(dir: &std::path::Path) -> Option<PathBuf> {
    let explicit = dir.join("hs_err.log");
    if explicit.exists() {
        return Some(explicit);
    }
    let mut paths = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("hs_err_pid") && name.ends_with(".log") {
                Some(e.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.pop()
}
