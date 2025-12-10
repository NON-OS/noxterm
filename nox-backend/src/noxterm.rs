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
use std::path::Path as StdPath;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

mod anyone_service;
use anyone_service::AnyoneService;

/// Cross-platform Docker connection with automatic setup
async fn connect_docker() -> Result<Docker> {
    // Check for explicit DOCKER_HOST environment variable first
    if let Ok(docker_host) = std::env::var("DOCKER_HOST") {
        info!("Using DOCKER_HOST: {}", docker_host);
        return Docker::connect_with_local_defaults()
            .map_err(|e| anyhow::anyhow!("Docker connection failed with DOCKER_HOST={}: {}", docker_host, e));
    }

    let home = std::env::var("HOME").unwrap_or_default();

    // Platform-specific socket paths to try
    let socket_paths: Vec<String> = if cfg!(target_os = "macos") {
        vec![
            "/var/run/docker.sock".to_string(),
            format!("{}/.docker/run/docker.sock", home),
            "/Users/Shared/docker/docker.sock".to_string(),
            format!("{}/.orbstack/run/docker.sock", home),
            format!("{}/.colima/default/docker.sock", home),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            "npipe:////./pipe/docker_engine".to_string(),
        ]
    } else {
        vec![
            "/var/run/docker.sock".to_string(),
            "/run/docker.sock".to_string(),
            format!("{}/.docker/run/docker.sock", home),
        ]
    };

    // First attempt: try to connect to existing Docker
    if let Some(docker) = try_connect_docker(&socket_paths) {
        return Ok(docker);
    }

    // No Docker running - try to start or install it
    info!("Docker not running. Attempting to start or install...");

    if cfg!(target_os = "macos") {
        // Try to start existing Docker installations
        if try_start_docker_macos().await {
            // Wait for Docker to be ready
            for i in 1..=30 {
                info!("Waiting for Docker to start... ({}/30)", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if let Some(docker) = try_connect_docker(&socket_paths) {
                    info!("Docker started successfully!");
                    return Ok(docker);
                }
            }
        }

        // No Docker installed - install Colima (lightweight, free)
        info!("No Docker runtime found. Installing Colima (lightweight Docker runtime)...");
        if install_and_start_colima().await? {
            // Wait for Colima to fully start (it takes time to set up Docker)
            info!("Waiting for Colima and Docker to be fully ready...");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            // Update socket path for Colima
            let colima_socket = format!("{}/.colima/default/docker.sock", home);
            for i in 1..=90 {
                info!("Waiting for Docker socket... ({}/90)", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                if StdPath::new(&colima_socket).exists() {
                    // Socket exists, try to connect
                    match Docker::connect_with_unix(&colima_socket, 120, bollard::API_DEFAULT_VERSION) {
                        Ok(docker) => {
                            // Verify it actually works by pinging
                            match docker.ping().await {
                                Ok(_) => {
                                    info!("Docker is ready!");
                                    return Ok(docker);
                                }
                                Err(_) => {
                                    debug!("Docker socket exists but not responding yet...");
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Cannot connect to socket yet: {}", e);
                        }
                    }
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        // Try to start Docker daemon (will also auto-install if needed)
        if try_start_docker_linux().await {
            for i in 1..=30 {
                info!("Waiting for Docker to start... ({}/30)", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if let Some(docker) = try_connect_docker(&socket_paths) {
                    info!("Docker started successfully!");
                    return Ok(docker);
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        // Try to start Docker Desktop on Windows
        if try_start_docker_windows().await {
            for i in 1..=60 {
                info!("Waiting for Docker Desktop to start... ({}/60)", i);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if let Some(docker) = try_connect_docker(&socket_paths) {
                    info!("Docker started successfully!");
                    return Ok(docker);
                }
            }
        }
    }

    // Final attempt
    if let Some(docker) = try_connect_docker(&socket_paths) {
        return Ok(docker);
    }

    // Platform-specific error message
    let install_instructions = if cfg!(target_os = "macos") {
        "macOS: Run 'brew install colima docker && colima start'"
    } else if cfg!(target_os = "linux") {
        "Linux: Run 'curl -fsSL https://get.docker.com | sudo sh && sudo systemctl start docker'"
    } else if cfg!(target_os = "windows") {
        "Windows: Download and install Docker Desktop from https://docker.com/products/docker-desktop"
    } else {
        "Please install Docker for your platform from https://docker.com"
    };

    Err(anyhow::anyhow!(
        "Failed to connect to Docker.\n\n{}\n\nAfter installation, restart NOXTERM.",
        install_instructions
    ))
}

fn try_connect_docker(socket_paths: &[String]) -> Option<Docker> {
    for socket_path in socket_paths {
        if socket_path.is_empty() {
            continue;
        }

        if !socket_path.starts_with("npipe:") && !StdPath::new(socket_path).exists() {
            continue;
        }

        if let Ok(docker) = Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION) {
            info!("Connected to Docker at: {}", socket_path);
            return Some(docker);
        }
    }

    // Try default connection
    Docker::connect_with_local_defaults().ok()
}

async fn try_start_docker_macos() -> bool {
    use std::process::Command;

    // Try Docker Desktop
    if StdPath::new("/Applications/Docker.app").exists() {
        info!("Starting Docker Desktop...");
        let _ = Command::new("open").args(["-a", "Docker"]).spawn();
        return true;
    }

    // Try OrbStack
    if Command::new("which").arg("orbctl").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Starting OrbStack...");
        let _ = Command::new("orbctl").arg("start").spawn();
        return true;
    }

    // Try Colima
    if Command::new("which").arg("colima").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Starting Colima...");
        let _ = Command::new("colima").arg("start").spawn();
        return true;
    }

    false
}

async fn try_start_docker_linux() -> bool {
    use std::process::Command;

    // Check if Docker is installed
    let docker_installed = Command::new("which")
        .arg("docker")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !docker_installed {
        info!("Docker not installed. Attempting to install...");
        if install_docker_linux().await {
            info!("Docker installed successfully!");
        } else {
            warn!("Failed to auto-install Docker. Please install manually:");
            warn!("  curl -fsSL https://get.docker.com | sudo sh");
            return false;
        }
    }

    // Try systemctl (most common on modern Linux)
    info!("Starting Docker daemon via systemctl...");
    if Command::new("sudo")
        .args(["systemctl", "start", "docker"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        // Also enable it for future boots
        let _ = Command::new("sudo")
            .args(["systemctl", "enable", "docker"])
            .status();
        return true;
    }

    // Try service command (older systems)
    info!("Trying service command...");
    if Command::new("sudo")
        .args(["service", "docker", "start"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }

    // Try starting dockerd directly
    info!("Trying to start dockerd directly...");
    if Command::new("sudo")
        .args(["dockerd", "&"])
        .spawn()
        .is_ok()
    {
        return true;
    }

    false
}

async fn install_docker_linux() -> bool {
    use std::process::Command;

    // Detect package manager and install Docker
    // Try the official Docker install script (works on most distros)
    info!("Running Docker install script...");
    let install_result = Command::new("sh")
        .args(["-c", "curl -fsSL https://get.docker.com | sudo sh"])
        .status();

    if install_result.map(|s| s.success()).unwrap_or(false) {
        // Add current user to docker group
        if let Ok(user) = std::env::var("USER") {
            let _ = Command::new("sudo")
                .args(["usermod", "-aG", "docker", &user])
                .status();
            info!("Added user {} to docker group", user);
        }
        return true;
    }

    // Fallback: try apt-get (Debian/Ubuntu)
    if Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Installing Docker via apt-get...");
        let _ = Command::new("sudo").args(["apt-get", "update"]).status();
        if Command::new("sudo")
            .args(["apt-get", "install", "-y", "docker.io"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    // Fallback: try dnf (Fedora/RHEL)
    if Command::new("which").arg("dnf").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Installing Docker via dnf...");
        if Command::new("sudo")
            .args(["dnf", "install", "-y", "docker"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    // Fallback: try yum (CentOS/older RHEL)
    if Command::new("which").arg("yum").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Installing Docker via yum...");
        if Command::new("sudo")
            .args(["yum", "install", "-y", "docker"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    // Fallback: try pacman (Arch)
    if Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false) {
        info!("Installing Docker via pacman...");
        if Command::new("sudo")
            .args(["pacman", "-S", "--noconfirm", "docker"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

async fn try_start_docker_windows() -> bool {
    use std::process::Command;

    // Common Docker Desktop installation paths on Windows
    let docker_desktop_paths = [
        r"C:\Program Files\Docker\Docker\Docker Desktop.exe",
        r"C:\Program Files (x86)\Docker\Docker\Docker Desktop.exe",
    ];

    // Check if Docker Desktop is installed
    for path in &docker_desktop_paths {
        if StdPath::new(path).exists() {
            info!("Found Docker Desktop at: {}", path);
            info!("Starting Docker Desktop...");

            // Start Docker Desktop
            if Command::new(path).spawn().is_ok() {
                info!("Docker Desktop starting... (this may take 30-60 seconds)");
                return true;
            }
        }
    }

    // Try using 'start' command which might find it in PATH
    if Command::new("cmd")
        .args(["/c", "start", "", "Docker Desktop"])
        .spawn()
        .is_ok()
    {
        info!("Docker Desktop starting via start command...");
        return true;
    }

    // Check if docker CLI is available (might be WSL2 backend)
    if Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }

    warn!("Docker Desktop not found. Please install from: https://docker.com/products/docker-desktop");
    warn!("After installation, restart NOXTERM.");
    false
}

async fn install_and_start_colima() -> Result<bool> {
    use std::process::Command;

    // Check if Homebrew is installed
    let brew_installed = Command::new("which")
        .arg("brew")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !brew_installed {
        warn!("Homebrew not installed. Cannot auto-install Colima.");
        info!("Install Homebrew first: /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"");
        return Ok(false);
    }

    // Check if docker CLI is installed
    let docker_cli_installed = Command::new("which")
        .arg("docker")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !docker_cli_installed {
        info!("Installing Docker CLI...");
        let status = Command::new("brew")
            .args(["install", "docker"])
            .status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            warn!("Failed to install Docker CLI");
            return Ok(false);
        }
    }

    // Check if Colima is installed
    let colima_installed = Command::new("which")
        .arg("colima")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !colima_installed {
        info!("Installing Colima (this may take a few minutes)...");
        let status = Command::new("brew")
            .args(["install", "colima"])
            .status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            warn!("Failed to install Colima");
            return Ok(false);
        }
        info!("Colima installed successfully!");
    }

    // Start Colima
    info!("Starting Colima...");
    let _ = Command::new("colima")
        .args(["start", "--cpu", "2", "--memory", "4"])
        .spawn();

    Ok(true)
}

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

    let mut last_activity = std::time::Instant::now();
    let idle_timeout = std::time::Duration::from_secs(600); // 10 min idle timeout for command mode

    loop {
        // Use timeout to allow periodic keepalive checks
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            ws_receiver.next()
        ).await;

        let msg = match msg {
            Ok(Some(msg)) => msg,
            Ok(None) => {
                info!("WebSocket stream ended for session {}", session_id);
                break;
            }
            Err(_) => {
                // Timeout - check idle time and send keepalive
                if last_activity.elapsed() > idle_timeout {
                    warn!("Session {} idle timeout (10 min)", session_id);
                    let _ = ws_sender.send(Message::Text(
                        serde_json::json!({
                            "type": "session_timeout",
                            "message": "Session timed out due to inactivity"
                        }).to_string()
                    )).await;
                    break;
                }
                // Send ping to keep connection alive
                if ws_sender.send(Message::Ping(vec![1, 2, 3, 4])).await.is_err() {
                    info!("Ping failed - client disconnected");
                    break;
                }
                continue;
            }
        };

        match msg {
            Ok(Message::Text(command)) => {
                last_activity = std::time::Instant::now();
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
            Ok(Message::Ping(_)) => {
                last_activity = std::time::Instant::now();
                // Pong is sent automatically by axum
            },
            Ok(Message::Pong(_)) => {
                last_activity = std::time::Instant::now();
            },
            Ok(Message::Binary(_)) => {
                last_activity = std::time::Instant::now();
                // Binary messages not used in command mode
            },
            Err(e) => {
                error!("WebSocket error for session {}: {}", session_id, e);
                break;
            }
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
    use bollard::exec::{CreateExecOptions, StartExecOptions, ResizeExecOptions};
    use tokio::sync::mpsc;

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

    // Create a proper interactive shell with full PTY support
    // Use bash with login + interactive flags for proper terminal setup
    let exec_config = CreateExecOptions {
        cmd: Some(vec![
            "/bin/bash".to_string(),
            "--login".to_string(),
            "-i".to_string(),
        ]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(true),
        tty: Some(true),
        env: Some(vec![
            "TERM=xterm-256color".to_string(),
            "COLORTERM=truecolor".to_string(),
            "DEBIAN_FRONTEND=noninteractive".to_string(),
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            "HOME=/root".to_string(),
            "SHELL=/bin/bash".to_string(),
            "USER=root".to_string(),
            "LANG=en_US.UTF-8".to_string(),
            "LC_ALL=en_US.UTF-8".to_string(),
            "LC_CTYPE=en_US.UTF-8".to_string(),
            // Nano-specific settings
            "EDITOR=nano".to_string(),
            "VISUAL=nano".to_string(),
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

    // Resize the PTY to default terminal size AFTER starting (exec must be running)
    // Give it a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let resize_result = state.docker.resize_exec(&exec_id, ResizeExecOptions {
        height: 24,
        width: 80,
    }).await;
    if let Err(e) = resize_result {
        debug!("Initial PTY resize warning: {} (non-fatal)", e);
    }

    // Send ready message
    let _ = ws_sender.send(Message::Text(
        "\x1b[2J\x1b[H\r\n🥷 NØXTERM PTY Ready!\r\n\r\n\
         Editor shortcuts:\r\n\
         • nano: Ctrl+O (save), Ctrl+X (exit), Ctrl+W (search)\r\n\
         • vim:  :w (save), :q (quit), :wq (save+quit), ESC (normal mode)\r\n\
         • cd, ls, cat, etc. all work normally\r\n\r\n".to_string()
    )).await;

    match exec_stream {
        bollard::exec::StartExecResults::Attached { mut output, mut input } => {
            // Use channels for graceful shutdown coordination
            let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
            let shutdown_tx2 = shutdown_tx.clone();

            // Channel for resize requests (exec_id needed in input task)
            let (resize_tx, mut resize_rx) = mpsc::channel::<(u16, u16)>(4);
            let exec_id_clone = exec_id.clone();
            let docker_clone = state.docker.clone();

            // Spawn resize handler task
            let resize_task = tokio::spawn(async move {
                while let Some((cols, rows)) = resize_rx.recv().await {
                    let resize_result = docker_clone.resize_exec(&exec_id_clone, ResizeExecOptions {
                        height: rows,
                        width: cols,
                    }).await;
                    if let Err(e) = resize_result {
                        debug!("PTY resize to {}x{} warning: {}", cols, rows, e);
                    } else {
                        debug!("PTY resized to {}x{}", cols, rows);
                    }
                }
            });

            // Handle input from WebSocket to container stdin
            let input_task = tokio::spawn(async move {
                let mut last_activity = std::time::Instant::now();
                let idle_timeout = std::time::Duration::from_secs(600); // 10 min idle timeout for PTY

                loop {
                    tokio::select! {
                        // Check for shutdown signal
                        _ = shutdown_rx.recv() => {
                            debug!("Input task received shutdown signal");
                            break;
                        }
                        // Wait for WebSocket message with timeout
                        msg = tokio::time::timeout(std::time::Duration::from_secs(30), ws_receiver.next()) => {
                            match msg {
                                Ok(Some(Ok(Message::Text(text)))) => {
                                    last_activity = std::time::Instant::now();

                                    // Check for resize command (JSON format: {"resize": [cols, rows]})
                                    if text.starts_with("{\"resize\":") {
                                        if let Ok(resize_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                                            if let Some(arr) = resize_msg.get("resize").and_then(|v| v.as_array()) {
                                                if arr.len() == 2 {
                                                    let cols = arr[0].as_u64().unwrap_or(80) as u16;
                                                    let rows = arr[1].as_u64().unwrap_or(24) as u16;
                                                    debug!("Resizing PTY to {}x{}", cols, rows);
                                                    let _ = resize_tx.send((cols, rows)).await;
                                                }
                                            }
                                        }
                                        continue;
                                    }

                                    // Log the input for debugging
                                    debug!("PTY input received: {:?} ({} bytes)",
                                        text.chars().take(20).collect::<String>(),
                                        text.len());

                                    // Write raw terminal input to container stdin
                                    match input.write_all(text.as_bytes()).await {
                                        Ok(_) => {
                                            // Flush immediately to ensure data is sent
                                            if let Err(e) = input.flush().await {
                                                warn!("Failed to flush PTY stdin: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to write to PTY stdin: {}", e);
                                            break;
                                        }
                                    }
                                }
                                Ok(Some(Ok(Message::Binary(data)))) => {
                                    last_activity = std::time::Instant::now();

                                    // Binary data is raw terminal input - pass through directly
                                    if input.write_all(&data).await.is_err() {
                                        warn!("Failed to write binary to PTY stdin");
                                        break;
                                    }
                                    let _ = input.flush().await;
                                }
                                Ok(Some(Ok(Message::Ping(data)))) => {
                                    last_activity = std::time::Instant::now();
                                    debug!("Received ping, activity refreshed");
                                    let _ = data;
                                }
                                Ok(Some(Ok(Message::Pong(_)))) => {
                                    last_activity = std::time::Instant::now();
                                }
                                Ok(Some(Ok(Message::Close(_)))) => {
                                    info!("PTY WebSocket closed by client");
                                    break;
                                }
                                Ok(Some(Err(e))) => {
                                    warn!("PTY WebSocket error: {}", e);
                                    break;
                                }
                                Ok(None) => {
                                    info!("PTY WebSocket stream ended");
                                    break;
                                }
                                Err(_) => {
                                    // Timeout - check idle time
                                    if last_activity.elapsed() > idle_timeout {
                                        warn!("PTY session idle timeout (10 min)");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                debug!("PTY input handler finished");
                let _ = shutdown_tx.send(()).await;
            });

            // Handle output from container stdout to WebSocket
            let output_task = tokio::spawn(async move {
                let mut consecutive_errors = 0;
                let max_consecutive_errors = 5;
                debug!("PTY output handler started");

                loop {
                    // Read with timeout to allow periodic checks
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(60),
                        output.next()
                    ).await {
                        Ok(Some(Ok(log_output))) => {
                            consecutive_errors = 0; // Reset on success
                            let data = match log_output {
                                bollard::container::LogOutput::StdOut { message } => {
                                    debug!("PTY stdout: {} bytes", message.len());
                                    message
                                },
                                bollard::container::LogOutput::StdErr { message } => {
                                    debug!("PTY stderr: {} bytes", message.len());
                                    message
                                },
                                bollard::container::LogOutput::Console { message } => {
                                    debug!("PTY console: {} bytes", message.len());
                                    message
                                },
                                bollard::container::LogOutput::StdIn { .. } => {
                                    debug!("PTY stdin echo (ignored)");
                                    continue;
                                }
                            };

                            // Send binary data directly to preserve escape sequences
                            if ws_sender.send(Message::Binary(data.into())).await.is_err() {
                                info!("WebSocket send failed - client disconnected");
                                break;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            consecutive_errors += 1;
                            warn!("PTY output error ({}/{}): {}", consecutive_errors, max_consecutive_errors, e);
                            if consecutive_errors >= max_consecutive_errors {
                                error!("Too many consecutive PTY errors, closing connection");
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                        Ok(None) => {
                            info!("PTY output stream ended (shell exited)");
                            let _ = ws_sender.send(Message::Text("\r\n\r\n[Shell exited]\r\n".to_string())).await;
                            break;
                        }
                        Err(_) => {
                            // Timeout - send ping to keep connection alive
                            if ws_sender.send(Message::Ping(vec![1, 2, 3, 4])).await.is_err() {
                                info!("Ping failed - client disconnected");
                                break;
                            }
                        }
                    }
                }
                debug!("PTY output handler finished");
                let _ = shutdown_tx2.send(()).await;
            });

            // Wait for all tasks to complete
            let (input_result, output_result, _) = tokio::join!(input_task, output_task, resize_task);

            if let Err(e) = input_result {
                warn!("Input task panicked: {}", e);
            }
            if let Err(e) = output_result {
                warn!("Output task panicked: {}", e);
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
    use bollard::image::CreateImageOptions;

    let session = {
        let sessions = state.sessions.read().await;
        sessions.get(&session_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?
    };

    let image = session.container_image.clone();
    let container_name = format!("noxterm-{}", session_id.to_string().replace("-", "")[0..12].to_lowercase());

    // Auto-pull image if not present
    info!("Checking for image: {}", image);
    let images = docker.list_images::<String>(None).await?;
    let image_exists = images.iter().any(|img| {
        img.repo_tags.iter().any(|tag| tag.contains(&image) || tag == &image)
    });

    if !image_exists {
        info!("Image {} not found locally, pulling...", image);

        let options = CreateImageOptions {
            from_image: image.as_str(),
            ..Default::default()
        };

        let mut stream = docker.create_image(Some(options), None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!("Pull progress: {}", status);
                    }
                }
                Err(e) => {
                    error!("Failed to pull image {}: {}", image, e);
                    return Err(anyhow::anyhow!("Failed to pull image {}: {}", image, e));
                }
            }
        }
        info!("Successfully pulled image: {}", image);
    }

    let config = Config {
        image: Some(image),
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
    use tracing_subscriber::EnvFilter;

    // Use RUST_LOG if set, otherwise default to info level
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("noxterm=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
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

    // Connect to Docker with cross-platform support (auto-installs if needed)
    let docker = connect_docker().await?;

    let version = docker.version().await
        .map_err(|e| anyhow::anyhow!("Docker daemon not responding. Is Docker running?\nError: {}", e))?;

    info!("✅ Docker connected successfully");
    info!("Docker version: {}", version.version.unwrap_or_else(|| "unknown".to_string()));
    info!("Platform: {} / {}", std::env::consts::OS, std::env::consts::ARCH);

    // Initialize Anyone Protocol service with auto-install
    let anyone_service = Arc::new(AnyoneService::new(9050, 9051));
    info!("🔐 Anyone Protocol service initialized (SOCKS: 9050, Control: 9051)");

    // Pre-install Anyone SDK in background (don't block startup)
    let anyone_clone = anyone_service.clone();
    tokio::spawn(async move {
        if let Err(e) = anyone_clone.ensure_prerequisites().await {
            warn!("Anyone Protocol prerequisites check: {}", e);
        } else {
            info!("✅ Anyone Protocol SDK ready");
        }
    });

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
