#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use russh::client::{self, Config};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::command;

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

struct SimpleHandler;

impl client::Handler for SimpleHandler {
    type Error = russh::Error;
    
    async fn check_server_key(&mut self, _server_public_key: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[command]
async fn load_connections() -> Result<Vec<Connection>, String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("获取可执行文件路径失败: {}", e))?;
    let config_path = exe_path.parent()
        .ok_or("无法获取可执行文件目录".to_string())?
        .join("pwd.json");
    
    if !config_path.exists() {
        return Err(format!("配置文件不存在: {}", config_path.display()));
    }
    
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;
    
    let connections: Vec<Connection> = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;
    
    Ok(connections)
}

#[command]
async fn connect_ssh(
    ip: String,
    port: u16,
    username: String,
    password: String,
) -> Result<String, String> {
    let config = Arc::new(Config::default());
    let mut sh = client::connect(config, (ip.as_str(), port), SimpleHandler).await
        .map_err(|e| format!("连接失败: {}", e))?;

    let auth_result = sh.authenticate_password(username.as_str(), password.as_str()).await
        .map_err(|e| format!("认证失败: {}", e))?;

    match auth_result {
        client::AuthResult::Success => Ok("SSH 连接成功".to_string()),
        _ => Err("认证失败: 用户名或密码错误".to_string()),
    }
}

#[command]
async fn execute_command(
    ip: String,
    port: u16,
    username: String,
    password: String,
    command: String,
) -> Result<String, String> {
    let config = Arc::new(Config::default());
    let mut sh = client::connect(config, (ip.as_str(), port), SimpleHandler).await
        .map_err(|e| format!("连接失败: {}", e))?;

    let auth_result = sh.authenticate_password(username.as_str(), password.as_str()).await
        .map_err(|e| format!("认证失败: {}", e))?;

    if !matches!(auth_result, client::AuthResult::Success) {
        return Err("认证失败: 用户名或密码错误".to_string());
    }

    let mut channel = sh.channel_open_session().await
        .map_err(|e| format!("打开通道失败: {}", e))?;

    let exec_result = channel.exec(true, command.as_str()).await;
    if let Err(e) = exec_result {
        return Err(format!("执行命令失败: {}", e));
    }

    let mut output = String::new();
    let mut exit_status: Option<u32> = None;
    
    loop {
        let msg = channel.wait().await;
        
        if msg.is_none() {
            break;
        }
        
        match msg.unwrap() {
            russh::ChannelMsg::Data { ref data } => {
                output.push_str(&String::from_utf8_lossy(data));
            }
            russh::ChannelMsg::ExitStatus { exit_status: status } => {
                exit_status = Some(status);
            }
            russh::ChannelMsg::Eof => {
                break;
            }
            _ => {}
        }
    }

    if let Some(status) = exit_status {
        if status != 0 {
            return Err(format!("命令执行失败，退出码: {}", status));
        }
    }

    channel.close().await
        .map_err(|e| format!("关闭通道失败: {}", e))?;

    Ok(output)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_connections,
            connect_ssh,
            execute_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
