use anyhow::Result;
use axum::{
    extract::{State, WebSocketUpgrade, Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use tokio::io::AsyncWriteExt;
use bollard::{Docker, container::{CreateContainerOptions, Config, StartContainerOptions}};
use bollard::models::HostConfig;
use futures::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

mod anyone_service;
use anyone_service::AnyoneService;

#[derive(Clone)]
struct AppState {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    docker: Arc<Docker>,
    config: AppConfig,
    anyone_service: Arc<AnyoneService>,
}

#[derive(Clone, Debug)]
struct AppConfig {
    host: String,
    port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Session {
    id: Uuid,
    user_id: String,
    status: String,
    container_id: Option<String>,
    container_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    container_image: String,
}


#[derive(Deserialize)]
struct CreateSessionRequest {
    user_id: String,
    container_image: Option<String>,
}

#[derive(Serialize)]
struct CreateSessionResponse {
    session_id: Uuid,
    websocket_url: String,
    status: String,
}

#[derive(Serialize)]
struct PrivacyStatusResponse {
    enabled: bool,
    socks_port: Option<u16>,
    control_port: Option<u16>,
    status: String,
}

#[derive(Serialize)]
struct PrivacyResponse {
    status: String,
    socks_port: Option<u16>,
    message: String,
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "noxterm-production",
        "version": env!("CARGO_PKG_VERSION"),
        "build_time": env!("BUILD_TIME"),
        "git_hash": env!("GIT_HASH"),
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        "timestamp": chrono::Utc::now(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

// Create session endpoint
async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let session_id = Uuid::new_v4();
    let session = Session {
        id: session_id,
        user_id: payload.user_id.clone(),
        status: "created".to_string(),
        container_id: None,
        container_name: None,
        created_at: chrono::Utc::now(),
        container_image: payload.container_image.unwrap_or_else(|| "ubuntu:22.04".to_string()),
    };

    let websocket_url = format!("ws://{}:{}/ws/{}", state.config.host, state.config.port, session_id);

    // Store session
    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id, session);
    }

    info!("Created session {} for user {}", session_id, payload.user_id);

    let response = CreateSessionResponse {
        session_id,
        websocket_url,
        status: "created".to_string(),
    };

    Ok(Json(response))
}

// Get session endpoint
async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let sessions = state.sessions.read().await;
    
    match sessions.get(&session_id) {
        Some(session) => Ok(Json(session.clone())),
        None => {
            warn!("Session {} not found", session_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// List sessions endpoint
async fn list_sessions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let sessions = state.sessions.read().await;
    let user_id = params.get("user_id");
    
    let filtered_sessions: Vec<&Session> = sessions
        .values()
        .filter(|session| {
            user_id.map_or(true, |uid| &session.user_id == uid)
        })
        .collect();

    Json(serde_json::json!({
        "sessions": filtered_sessions,
        "count": filtered_sessions.len()
    }))
}

// Privacy control endpoints

// Enable privacy mode (start Anyone service)
async fn enable_privacy(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Enabling privacy mode...");
    
    match state.anyone_service.start().await {
        Ok(_) => {
            let socks_port = state.anyone_service.get_socks_port();
            info!("Privacy mode enabled successfully on SOCKS port {}", socks_port);
            
            let response = PrivacyResponse {
                status: "enabled".to_string(),
                socks_port: Some(socks_port),
                message: "Anyone Protocol network activated".to_string(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to enable privacy mode: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Disable privacy mode (stop Anyone service)
async fn disable_privacy(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Disabling privacy mode...");
    
    match state.anyone_service.stop().await {
        Ok(_) => {
            info!("Privacy mode disabled successfully");
            
            let response = PrivacyResponse {
                status: "disabled".to_string(),
                socks_port: None,
                message: "Anyone Protocol network deactivated".to_string(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to disable privacy mode: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Get privacy status
async fn privacy_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let enabled = state.anyone_service.is_enabled().await;
    let service_status = state.anyone_service.get_status().await;
    
    let response = PrivacyStatusResponse {
        enabled,
        socks_port: if enabled { Some(state.anyone_service.get_socks_port()) } else { None },
        control_port: if enabled { Some(state.anyone_service.get_control_port()) } else { None },
        status: format!("{:?}", service_status),
    };
    
    Json(response)
}

// WebSocket handler with working terminal
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("WebSocket connection request for session {}", session_id);
    
    // Check if session exists
    {
        let sessions = state.sessions.read().await;
        if !sessions.contains_key(&session_id) {
            warn!("WebSocket connection rejected - session {} not found", session_id);
            return (StatusCode::NOT_FOUND, "Session not found").into_response();
        }
    }

    ws.on_upgrade(move |socket| handle_websocket(socket, session_id, state))
}

async fn pty_websocket_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<Uuid>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("PTY WebSocket connection request for session {}", session_id);
    
    let sessions = state.sessions.read().await;
    if !sessions.contains_key(&session_id) {
        error!("Session {} not found for PTY WebSocket", session_id);
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }
    drop(sessions);

    ws.on_upgrade(move |socket| handle_pty_websocket(socket, session_id, state))
}

async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    session_id: Uuid,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};

    info!("WebSocket connected for session {}", session_id);
    
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Start a Docker container with exec
    let container_id = match start_container(&state.docker, session_id, &state).await {
        Ok((container_id, container_name)) => {
            info!("Started container {} for session {}", container_name, session_id);
            
            // Update session
            {
                let mut sessions = state.sessions.write().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.container_id = Some(container_id.clone());
                    session.container_name = Some(container_name.clone());
                    session.status = "running".to_string();
                }
            }
            
            // Send container ready message with working terminal
            if let Err(e) = ws_sender.send(Message::Text(
                serde_json::json!({
                    "type": "container_ready",
                    "session_id": session_id,
                    "container_id": container_id,
                    "container_name": container_name,
                    "message": "🐳 Container started! Terminal ready for commands.",
                    "timestamp": chrono::Utc::now()
                }).to_string()
            )).await {
                error!("Failed to send container ready message: {}", e);
                cleanup_container(&state, session_id).await;
                return;
            }
            
            container_id
        }
        Err(e) => {
            error!("Failed to start container for session {}: {}", session_id, e);
            
            if let Err(e) = ws_sender.send(Message::Text(
                serde_json::json!({
                    "type": "error",
                    "session_id": session_id,
                    "message": "Failed to start container",
                    "details": e.to_string()
                }).to_string()
            )).await {
                error!("Failed to send error message: {}", e);
            }
            return;
        }
    };

    if let Err(e) = ws_sender.send(Message::Text(
        serde_json::json!({
            "type": "terminal_ready",
            "session_id": session_id,
            "message": "🥷 TTY terminal ready! Interactive commands supported.",
            "features": [
                "TTY support enabled",
                "Extended timeouts for package operations",
                "Full UTF-8 locale support",
                "Error handling enabled"
            ],
            "timestamp": chrono::Utc::now()
        }).to_string()
    )).await {
        error!("Failed to send terminal ready message: {}", e);
        cleanup_container(&state, session_id).await;
        return;
    }

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(command)) => {
                if command.starts_with("\x1B[raw]") {
                    let raw_input = &command[6..];
                    debug!("Handling raw control input for session {}: {:?}", session_id, raw_input);
                    
                    match handle_interactive_input(&state.docker, &container_id, raw_input).await {
                        Ok(output) => {
                            if !output.trim().is_empty() {
                                let response = serde_json::json!({
                                    "type": "command_output", 
                                    "session_id": session_id,
                                    "command": format!("raw:{:?}", raw_input),
                                    "output": output,
                                    "raw_mode": true,
                                    "timestamp": chrono::Utc::now()
                                });
                                if ws_sender.send(Message::Text(response.to_string())).await.is_err() {
                                    break;
                                }
                            }
                        },
                        Err(e) => {
                            warn!("Raw input handling failed for session {}: {}", session_id, e);
                        }
                    }
                    continue;
                }
                
                let processed_command = if command.trim().starts_with("apt install") && !command.contains(" -y") {
                    format!("DEBIAN_FRONTEND=noninteractive apt install -y {}", command.trim().strip_prefix("apt install").unwrap_or("").trim())
                } else if command.trim().starts_with("apt-get install") && !command.contains(" -y") {
                    format!("DEBIAN_FRONTEND=noninteractive apt-get install -y {}", command.trim().strip_prefix("apt-get install").unwrap_or("").trim())
                } else if command.trim() == "apt update" {
                    "DEBIAN_FRONTEND=noninteractive apt update".to_string()
                } else if command.trim() == "apt upgrade" {
                    "DEBIAN_FRONTEND=noninteractive apt upgrade -y".to_string()
                } else {
                    command.clone()
                };
                
                debug!("Executing TTY command '{}' in session {}", processed_command, session_id);
                
                match execute_command_with_tty(&state.docker, &container_id, &processed_command).await {
                    Ok(output) => {
                        debug!("Command '{}' executed successfully in session {}", command, session_id);
                        
                        let response = serde_json::json!({
                            "type": "command_output",
                            "session_id": session_id,
                            "command": command,
                            "output": output,
                            "tty_enabled": true,
                            "timestamp": chrono::Utc::now()
                        });

                        if ws_sender.send(Message::Text(response.to_string())).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        error!("TTY command execution failed for '{}' in session {}: {}", command, session_id, e);
                        
                        let error_response = serde_json::json!({
                            "type": "command_error",
                            "session_id": session_id,
                            "command": command,
                            "error": e.to_string(),
                            "tty_enabled": true,
                            "timestamp": chrono::Utc::now()
                        });

                        if ws_sender.send(Message::Text(error_response.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
            },
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed for session {}", session_id);
                break;
            },
            Err(e) => {
                error!("WebSocket error for session {}: {}", session_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup container
    cleanup_container(&state, session_id).await;
}

async fn handle_interactive_input(
    docker: &Docker,
    container_id: &str,
    raw_input: &str,
) -> Result<String> {
    debug!("Processing interactive input: {:?}", raw_input);
    
    let control_char = raw_input.chars().next().unwrap_or('\0');
    let control_code = control_char as u32;
    
    let input_sequence = match control_code {
        24 => "\x18".to_string(), // Ctrl+X
        25 => "Y".to_string(),     // Y for save confirm
        13 => "\r".to_string(),    // Enter
        3 => "\x03".to_string(),   // Ctrl+C 
        26 => "\x1a".to_string(),  // Ctrl+Z
        27 => "\x1b".to_string(),  // ESC
        _ => raw_input.to_string(),
    };
    
    let exec = docker.create_exec(
        container_id,
        bollard::exec::CreateExecOptions {
            cmd: Some(vec!["/bin/bash", "-c", &format!("echo -ne '{}'" , input_sequence)]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            attach_stdin: Some(false),
            tty: Some(true),
            ..Default::default()
        },
    ).await?;

    let _result = docker.start_exec(&exec.id, None).await?;
    Ok("".to_string())
}

async fn handle_pty_websocket(
    socket: axum::extract::ws::WebSocket,
    session_id: Uuid,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use bollard::exec::{CreateExecOptions, StartExecOptions};
    
    info!("PTY WebSocket connected for session {}", session_id);
    
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let container_id = match start_container(&state.docker, session_id, &state).await {
        Ok((container_id, container_name)) => {
            info!("Started container {} for PTY session {}", container_name, session_id);
            
            {
                let mut sessions = state.sessions.write().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.container_id = Some(container_id.clone());
                    session.container_name = Some(container_name.clone());
                    session.status = "running".to_string();
                }
            }
            
            container_id
        }
        Err(e) => {
            error!("Failed to start container for session {}: {}", session_id, e);
            let _ = ws_sender.send(Message::Text(format!("\r\n❌ Container start failed: {}\r\n", e))).await;
            cleanup_container(&state, session_id).await;
            return;
        }
    };

    let exec_config = CreateExecOptions {
        cmd: Some(vec!["/bin/bash".to_string(), "-l".to_string()]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        tty: Some(true),
        env: Some(vec![
            "TERM=xterm-256color".to_string(),
            "DEBIAN_FRONTEND=noninteractive".to_string(),
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            "HOME=/root".to_string(),
            "SHELL=/bin/bash".to_string(),
            "LANG=en_US.UTF-8".to_string(),
            "LC_ALL=en_US.UTF-8".to_string(),
        ]),
        working_dir: Some("/root".to_string()),
        ..Default::default()
    };

    let exec_id = match state.docker.create_exec(&container_id, exec_config).await {
        Ok(exec) => exec.id,
        Err(e) => {
            error!("Failed to create PTY exec for session {}: {}", session_id, e);
            let _ = ws_sender.send(Message::Text(format!("\r\n❌ PTY creation failed: {}\r\n", e))).await;
            cleanup_container(&state, session_id).await;
            return;
        }
    };

    let exec_stream = match state.docker.start_exec(&exec_id, Some(StartExecOptions {
        tty: true,
        ..Default::default()
    })).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to start PTY exec for session {}: {}", session_id, e);
            let _ = ws_sender.send(Message::Text(format!("\r\n❌ PTY start failed: {}\r\n", e))).await;
            cleanup_container(&state, session_id).await;
            return;
        }
    };

    let _ = ws_sender.send(Message::Text("\r\n🥷 PTY terminal ready! Full editor support enabled.\r\n$ ".to_string())).await;

    match exec_stream {
        bollard::exec::StartExecResults::Attached { mut output, mut input } => {
            // Handle input - direct stdin handling  
            let input_task = tokio::spawn(async move {
                    while let Some(msg) = ws_receiver.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                debug!("PTY input: {:?}", text);
                                if input.write_all(text.as_bytes()).await.is_err() {
                                    break;
                                }
                                let _ = input.flush().await;
                            }
                            Ok(Message::Binary(data)) => {
                                debug!("PTY binary: {:?}", data.len());
                                if input.write_all(&data).await.is_err() {
                                    break;
                                }
                                let _ = input.flush().await;
                            }
                            Ok(Message::Close(_)) => {
                                debug!("PTY WebSocket closed");
                                break;
                            }
                            Err(e) => {
                                warn!("PTY input error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                    debug!("PTY input handler finished");
            });

            // Handle output
            let output_task = tokio::spawn(async move {
                while let Some(chunk) = output.next().await {
                    match chunk {
                        Ok(log_output) => {
                            let data = match log_output {
                                bollard::container::LogOutput::StdOut { message } => message,
                                bollard::container::LogOutput::StdErr { message } => message,
                                bollard::container::LogOutput::Console { message } => message,
                                _ => continue,
                            };
                            
                            if ws_sender.send(Message::Binary(data.into())).await.is_err() {
                                debug!("Failed to send PTY output");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("PTY output error: {}", e);
                            break;
                        }
                    }
                }
                debug!("PTY output handler finished");
            });

            // Wait for completion
            tokio::select! {
                _ = input_task => debug!("Input completed"),
                _ = output_task => debug!("Output completed"),
            }
        }
        bollard::exec::StartExecResults::Detached => {
            warn!("PTY exec detached mode not supported");
        }
    }

    info!("PTY WebSocket session {} completed", session_id);
    cleanup_container(&state, session_id).await;
}


async fn execute_command_with_tty(
    docker: &Docker,
    container_id: &str,
    command: &str,
) -> Result<String> {
    use bollard::exec::{CreateExecOptions, StartExecOptions};
    use futures::TryStreamExt;

    // Create a persistent bash session with TTY
    let exec = docker.create_exec(
        container_id,
        CreateExecOptions {
            cmd: Some(vec!["/bin/bash", "-c", command]),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            env: Some(vec![
                "DEBIAN_FRONTEND=noninteractive",
                "TERM=xterm-256color",
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "HOME=/root",
                "SHELL=/bin/bash",
                "USER=root",
                "LANG=en_US.UTF-8",
                "LC_ALL=en_US.UTF-8",
            ]),
            working_dir: Some("/root"),
            ..Default::default()
        },
    ).await?;

    let start_exec_options = StartExecOptions {
        detach: false,
        tty: true,
        ..Default::default()
    };

    match docker.start_exec(&exec.id, Some(start_exec_options)).await? {
        bollard::exec::StartExecResults::Attached { mut output, .. } => {
            let mut result = String::new();

            let timeout_duration = if command.contains("apt") || command.contains("git") || command.contains("wget") || command.contains("curl") {
                std::time::Duration::from_secs(300)
            } else if command.contains("nano") || command.contains("vim") || command.contains("emacs") {
                std::time::Duration::from_secs(30)
            } else {
                std::time::Duration::from_secs(60)
            };

            while let Ok(Ok(Some(chunk))) = tokio::time::timeout(timeout_duration, output.try_next()).await {
                match chunk {
                    bollard::container::LogOutput::StdOut { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::Console { message } => {
                        result.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }

            Ok(result)
        }
        bollard::exec::StartExecResults::Detached => {
            Ok("Command executed successfully (detached)".to_string())
        }
    }
}

async fn execute_command_in_container(
    docker: &Docker, 
    container_id: &str, 
    command: &str
) -> Result<String> {
    execute_command_with_tty(docker, container_id, command).await
}

async fn start_container(docker: &Docker, session_id: Uuid, state: &AppState) -> Result<(String, String)> {
    let session = {
        let sessions = state.sessions.read().await;
        sessions.get(&session_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?
    };

    let container_name = format!("noxterm-{}", session_id.to_string().replace("-", "")[0..12].to_lowercase());
    
    let config = Config {
        image: Some(session.container_image.clone()),
        cmd: Some(vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y nano vim curl wget git htop neofetch locales && locale-gen en_US.UTF-8 && update-locale LANG=en_US.UTF-8 && tail -f /dev/null".to_string()
        ]),
        env: Some(vec![
            "DEBIAN_FRONTEND=noninteractive".to_string(),
            "TERM=xterm-256color".to_string(),
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            "HOME=/root".to_string(),
            "SHELL=/bin/bash".to_string(),
            "LANG=en_US.UTF-8".to_string(),
            "LC_ALL=en_US.UTF-8".to_string(),
        ]),
        working_dir: Some("/root".to_string()),
        user: Some("root".to_string()),
        host_config: Some(HostConfig {
            memory: Some(1024 * 1024 * 1024), // 1GB memory
            memory_swap: Some(1024 * 1024 * 1024),
            cpu_quota: Some(100000), // 1 CPU
            cpu_period: Some(100000),
            pids_limit: Some(200),
            
            auto_remove: Some(true),
            privileged: Some(false),
            readonly_rootfs: Some(false),
            
            network_mode: Some("bridge".to_string()),
            
            cap_add: Some(vec![
                "SETUID".to_string(),
                "SETGID".to_string(),
                "CHOWN".to_string(),
                "DAC_OVERRIDE".to_string(),
                "FOWNER".to_string(),
            ]),
            
            ..Default::default()
        }),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: container_name.clone(),
        platform: None,
    };

    info!("Creating container {} for session {}", container_name, session_id);

    let response = docker.create_container(Some(options), config).await?;
    let container_id = response.id;

    docker.start_container(&container_id, None::<StartContainerOptions<String>>).await?;

    info!("Container {} started, waiting for setup completion", container_name);
    
    let mut retries = 40;
    while retries > 0 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        
        if let Ok(output) = execute_command_with_tty(docker, &container_id, "which nano && echo 'ready'").await {
            if output.contains("ready") {
                info!("Container {} setup completed", container_name);
                break;
            }
        }
        
        retries -= 1;
        if retries == 0 {
            warn!("Container setup timeout for {}, but continuing", container_name);
        }
    }

    Ok((container_id, container_name))
}

async fn cleanup_container(state: &AppState, session_id: Uuid) {
    let container_id = {
        let sessions = state.sessions.read().await;
        sessions.get(&session_id).and_then(|s| s.container_id.clone())
    };

    if let Some(container_id) = container_id {
        info!("Cleaning up container {} for session {}", container_id, session_id);
        
        if let Err(e) = state.docker.stop_container(&container_id, None).await {
            warn!("Failed to stop container {}: {}", container_id, e);
        }
        
        if let Err(e) = state.docker.remove_container(&container_id, None).await {
            warn!("Failed to remove container {}: {}", container_id, e);
        }
    }

    {
        let mut sessions = state.sessions.write().await;
        sessions.remove(&session_id);
    }
}

// Main application
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("noxterm=info,tower_http=info")
        .json()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    dotenvy::dotenv().ok();
    
    let config = AppConfig {
        host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid SERVER_PORT: {}", e))?,
    };

    info!("🥷 NOXTERM Backend Starting");
    info!("Host: {}", config.host);
    info!("Port: {}", config.port);
    info!("Environment: {}", std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()));

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| anyhow::anyhow!("Docker connection failed: {}", e))?;

    let version = docker.version().await
        .map_err(|e| anyhow::anyhow!("Docker version check failed: {}", e))?;

    info!("✅ Docker connected successfully");
    info!("Docker version: {}", version.version.unwrap_or_else(|| "unknown".to_string()));

    let anyone_service = Arc::new(AnyoneService::new(9050, 9051));
    info!("🔐 Anyone Protocol service initialized (SOCKS: 9050, Control: 9051)");

    let app_state = AppState {
        sessions: Arc::new(RwLock::new(HashMap::new())),
        docker: Arc::new(docker),
        config: config.clone(),
        anyone_service,
    };

    let app = Router::new()
        .route("/", get(|| async { Html("<h1>🥷 NOXTERM Backend</h1><p>Terminal service online</p>") }))
        .route("/health", get(health_check))
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/privacy/enable", post(enable_privacy))
        .route("/api/privacy/disable", post(disable_privacy))
        .route("/api/privacy/status", get(privacy_status))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/pty/:session_id", get(pty_websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;
    
    info!("✅ NOXTERM Backend Ready");
    
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
