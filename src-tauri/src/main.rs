#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    sync::Arc,
    thread,
};
use tauri::{async_runtime::Mutex as AsyncMutex, command, State};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Connection {
    id: Option<String>,
    label: String,
    ip: String,
    port: u16,
    #[serde(alias = "name")]
    username: String,
    #[serde(alias = "password")]
    pass: String,
}

struct AppState {
    pty_pair: Arc<AsyncMutex<Option<PtyPair>>>,
    writer: Arc<AsyncMutex<Option<Box<dyn Write + Send>>>>,
    reader: Arc<AsyncMutex<Option<BufReader<Box<dyn Read + Send>>>>>,
    current_connection: Arc<AsyncMutex<Option<Connection>>>,
}

#[command]
async fn load_connections() -> Result<Vec<Connection>, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取可执行文件路径失败: {}", e))?;
    let config_path = exe_path.parent()
        .ok_or("无法获取可执行文件目录".to_string())?
        .join("pwd.json");
    
    if !config_path.exists() {
        return Err(format!("配置文件不存在: {}", config_path.display()));
    }
    
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;
    
    let connections: Vec<Connection> = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;
    
    Ok(connections)
}

#[command]
async fn connect_ssh(
    state: State<'_, AppState>,
    ip: String,
    port: u16,
    username: String,
    password: String,
) -> Result<String, String> {
    let pty_system = native_pty_system();

    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("打开 PTY 失败: {}", e))?;

    let reader = pty_pair.master.try_clone_reader()
        .map_err(|e| format!("克隆读取器失败: {}", e))?;
    let writer = pty_pair.master.take_writer()
        .map_err(|e| format!("获取写入器失败: {}", e))?;

    let mut cmd = CommandBuilder::new("ssh");
    cmd.arg("-o");
    cmd.arg("StrictHostKeyChecking=no");
    cmd.arg("-p");
    cmd.arg(port.to_string());
    cmd.arg(format!("{}@{}", username, ip));

    #[cfg(target_os = "windows")]
    {
        cmd.env("TERM", "cygwin");
    }

    #[cfg(not(target_os = "windows"))]
    {
        cmd.env("TERM", "xterm-256color");
    }

    std::env::set_var("SSHPASS", &password);

    let mut child = pty_pair.slave.spawn_command(cmd)
        .map_err(|e| format!("启动 SSH 命令失败: {}", e))?;

    thread::spawn(move || {
        let _ = child.wait();
    });

    *state.pty_pair.lock().await = Some(pty_pair);
    *state.writer.lock().await = Some(writer);
    *state.reader.lock().await = Some(BufReader::new(reader));
    *state.current_connection.lock().await = Some(Connection {
        id: None,
        label: "".to_string(),
        ip: ip.clone(),
        port,
        username: username.clone(),
        pass: password,
    });

    Ok(format!("SSH 连接成功: {}@{}", username, ip))
}

#[command]
async fn disconnect_ssh(state: State<'_, AppState>) -> Result<String, String> {
    *state.pty_pair.lock().await = None;
    *state.writer.lock().await = None;
    *state.reader.lock().await = None;
    *state.current_connection.lock().await = None;
    Ok("SSH 连接已断开".to_string())
}

#[command]
async fn is_connected(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.current_connection.lock().await.is_some())
}

#[command]
async fn async_write_to_pty(data: &str, state: State<'_, AppState>) -> Result<(), String> {
    let mut writer = state.writer.lock().await;
    if let Some(ref mut w) = *writer {
        write!(w, "{}", data).map_err(|e| format!("写入失败: {}", e))
    } else {
        Err("未连接到服务器".to_string())
    }
}

#[command]
async fn async_read_from_pty(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut reader = state.reader.lock().await;
    
    if let Some(ref mut r) = *reader {
        let data = {
            let data = r.fill_buf().map_err(|e| format!("读取失败: {}", e))?;
            if data.len() > 0 {
                std::str::from_utf8(data)
                    .map(|v| Some(v.to_string()))
                    .map_err(|_| "UTF-8 解码失败".to_string())?
            } else {
                None
            }
        };

        if let Some(data) = &data {
            r.consume(data.len());
        }

        Ok(data)
    } else {
        Ok(None)
    }
}

#[command]
async fn async_resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), String> {
    let mut pty_pair = state.pty_pair.lock().await;
    if let Some(ref mut p) = *pty_pair {
        p.master.resize(PtySize {
            rows,
            cols,
            ..Default::default()
        }).map_err(|e| format!("调整大小失败: {}", e))
    } else {
        Err("未连接到服务器".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            pty_pair: Arc::new(AsyncMutex::new(None)),
            writer: Arc::new(AsyncMutex::new(None)),
            reader: Arc::new(AsyncMutex::new(None)),
            current_connection: Arc::new(AsyncMutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            load_connections,
            connect_ssh,
            disconnect_ssh,
            is_connected,
            async_write_to_pty,
            async_read_from_pty,
            async_resize_pty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
