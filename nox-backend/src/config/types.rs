// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! Configuration type definitions
//! All configuration structs and enums used throughout the application.

use std::net::SocketAddr;
use std::str::FromStr;

/// Main application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub docker: DockerConfig,
    pub session: SessionConfig,
    pub rate_limit: RateLimitConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
    pub anyone: AnyoneConfig,
}

/// Server binding configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub listen_addr: SocketAddr,
    pub environment: Environment,
    pub graceful_shutdown_timeout_secs: u64,
}

/// Environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Production,
    Staging,
    Development,
}

impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Ok(Environment::Production),
            "staging" | "stage" => Ok(Environment::Staging),
            "development" | "dev" | "" => Ok(Environment::Development),
            _ => Err(format!("Unknown environment: {}", s)),
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Production => write!(f, "production"),
            Environment::Staging => write!(f, "staging"),
            Environment::Development => write!(f, "development"),
        }
    }
}

/// Docker/Container resource configuration
#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub cpu_shares: u64,
    pub cpu_quota: i64,
    pub cpu_period: u64,
    pub memory_limit_bytes: u64,
    pub memory_swap_bytes: i64,
    pub pids_limit: i64,
    pub allow_networking: bool,
    pub read_only_rootfs: bool,
    pub container_user: Option<String>,
    pub default_image: String,
    pub allowed_images: Vec<String>,
    pub stop_timeout_secs: u64,
    pub socket_path: Option<String>,
}

/// Session management configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_concurrent_sessions: u32,
    pub max_sessions_per_ip: u32,
    pub max_sessions_per_user: u32,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub grace_period_secs: u64,
    pub cleanup_interval_secs: u64,
    pub health_check_interval_secs: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub session_create_limit: u32,
    pub session_create_window_secs: u64,
    pub ws_message_limit: u32,
    pub api_request_limit: u32,
    pub global_limit: u32,
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: Option<String>,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub enabled: bool,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub validate_commands: bool,
    pub block_dangerous_commands: bool,
    pub log_security_events: bool,
    pub max_input_length: usize,
    pub trusted_proxies: Vec<String>,
    pub audit_logging: bool,
}

/// Observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub json_logs: bool,
    pub metrics_enabled: bool,
    pub metrics_path: String,
    pub tracing_enabled: bool,
}

/// Anyone Protocol configuration
#[derive(Debug, Clone)]
pub struct AnyoneConfig {
    pub enabled: bool,
    pub socks_port: u16,
    pub control_port: u16,
    pub auto_start: bool,
}
