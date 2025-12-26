// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//! Configuration loading from environment variables

use std::env;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};
use super::error::ConfigError;
use super::types::*;

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        if let Err(e) = dotenvy::dotenv() {
            if e.not_found() {
                info!("No .env file found, using environment variables only");
            } else {
                warn!("Error loading .env file: {}", e);
            }
        }

        let host = env_or("NOXTERM_HOST", "127.0.0.1");
        let port = env_parse("NOXTERM_PORT", 3001u16)?;
        let listen_addr =
            format!("{}:{}", host, port)
                .parse()
                .map_err(|e| ConfigError::InvalidValue {
                    key: "NOXTERM_HOST/PORT".to_string(),
                    value: format!("{}:{}", host, port),
                    reason: format!("Invalid socket address: {}", e),
                })?;

        let environment = env_parse("NOXTERM_ENVIRONMENT", Environment::Development)?;

        Ok(Config {
            server: ServerConfig {
                host: host.clone(),
                port,
                listen_addr,
                environment,
                graceful_shutdown_timeout_secs: env_parse("NOXTERM_SHUTDOWN_TIMEOUT", 30u64)?,
            },
            docker: DockerConfig {
                cpu_shares: env_parse("NOXTERM_DOCKER_CPU_SHARES", 512u64)?,
                cpu_quota: env_parse("NOXTERM_DOCKER_CPU_QUOTA", 50000i64)?,
                cpu_period: env_parse("NOXTERM_DOCKER_CPU_PERIOD", 100000u64)?,
                memory_limit_bytes: env_parse(
                    "NOXTERM_DOCKER_MEMORY_LIMIT",
                    512 * 1024 * 1024u64,
                )?,
                memory_swap_bytes: env_parse("NOXTERM_DOCKER_MEMORY_SWAP", -1i64)?,
                pids_limit: env_parse("NOXTERM_DOCKER_PIDS_LIMIT", 100i64)?,
                allow_networking: env_parse("NOXTERM_DOCKER_ALLOW_NETWORKING", false)?,
                read_only_rootfs: env_parse("NOXTERM_DOCKER_READ_ONLY_ROOTFS", false)?,
                container_user: env::var("NOXTERM_DOCKER_USER").ok(),
                default_image: env_or("NOXTERM_DOCKER_DEFAULT_IMAGE", "ubuntu:22.04"),
                allowed_images: env_list(
                    "NOXTERM_DOCKER_ALLOWED_IMAGES",
                    vec![
                        "ubuntu:24.04".to_string(),
                        "ubuntu:22.04".to_string(),
                        "ubuntu:20.04".to_string(),
                        "debian:12".to_string(),
                        "alpine:latest".to_string(),
                        "archlinux:latest".to_string(),
                    ],
                ),
                stop_timeout_secs: env_parse("NOXTERM_DOCKER_STOP_TIMEOUT", 10u64)?,
                socket_path: env::var("DOCKER_HOST")
                    .ok()
                    .or_else(|| env::var("NOXTERM_DOCKER_SOCKET").ok()),
            },
            session: SessionConfig {
                max_concurrent_sessions: env_parse("NOXTERM_MAX_SESSIONS", 100u32)?,
                max_sessions_per_ip: env_parse("NOXTERM_MAX_SESSIONS_PER_IP", 5u32)?,
                max_sessions_per_user: env_parse("NOXTERM_MAX_SESSIONS_PER_USER", 3u32)?,
                idle_timeout_secs: env_parse("NOXTERM_SESSION_IDLE_TIMEOUT", 600u64)?,
                max_lifetime_secs: env_parse("NOXTERM_SESSION_MAX_LIFETIME", 3600u64)?,
                grace_period_secs: env_parse("NOXTERM_SESSION_GRACE_PERIOD", 300u64)?,
                cleanup_interval_secs: env_parse("NOXTERM_CLEANUP_INTERVAL", 30u64)?,
                health_check_interval_secs: env_parse("NOXTERM_HEALTH_CHECK_INTERVAL", 30u64)?,
            },
            rate_limit: RateLimitConfig {
                enabled: env_parse("NOXTERM_RATE_LIMIT_ENABLED", true)?,
                session_create_limit: env_parse("NOXTERM_RATE_LIMIT_SESSION_CREATE", 10u32)?,
                session_create_window_secs: env_parse("NOXTERM_RATE_LIMIT_SESSION_WINDOW", 60u64)?,
                ws_message_limit: env_parse("NOXTERM_RATE_LIMIT_WS_MESSAGES", 100u32)?,
                api_request_limit: env_parse("NOXTERM_RATE_LIMIT_API", 100u32)?,
                global_limit: env_parse("NOXTERM_RATE_LIMIT_GLOBAL", 1000u32)?,
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .ok()
                    .or_else(|| env::var("NOXTERM_DATABASE_URL").ok()),
                max_connections: env_parse("NOXTERM_DB_MAX_CONNECTIONS", 20u32)?,
                min_connections: env_parse("NOXTERM_DB_MIN_CONNECTIONS", 2u32)?,
                connect_timeout_secs: env_parse("NOXTERM_DB_CONNECT_TIMEOUT", 10u64)?,
                idle_timeout_secs: env_parse("NOXTERM_DB_IDLE_TIMEOUT", 600u64)?,
                enabled: env::var("DATABASE_URL").is_ok()
                    || env::var("NOXTERM_DATABASE_URL").is_ok(),
            },
            security: SecurityConfig {
                validate_commands: env_parse("NOXTERM_VALIDATE_COMMANDS", true)?,
                block_dangerous_commands: env_parse("NOXTERM_BLOCK_DANGEROUS_COMMANDS", true)?,
                log_security_events: env_parse("NOXTERM_LOG_SECURITY_EVENTS", true)?,
                max_input_length: env_parse("NOXTERM_MAX_INPUT_LENGTH", 10000usize)?,
                trusted_proxies: env_list(
                    "NOXTERM_TRUSTED_PROXIES",
                    vec!["127.0.0.1".to_string(), "::1".to_string()],
                ),
                audit_logging: env_parse("NOXTERM_AUDIT_LOGGING", true)?,
            },
            observability: ObservabilityConfig {
                log_level: env_or("NOXTERM_LOG_LEVEL", "info"),
                json_logs: env_parse("NOXTERM_JSON_LOGS", false)?,
                metrics_enabled: env_parse("NOXTERM_METRICS_ENABLED", true)?,
                metrics_path: env_or("NOXTERM_METRICS_PATH", "/metrics"),
                tracing_enabled: env_parse("NOXTERM_TRACING_ENABLED", true)?,
            },
            anyone: AnyoneConfig {
                enabled: env_parse("NOXTERM_ANYONE_ENABLED", true)?,
                socks_port: env_parse("NOXTERM_ANYONE_SOCKS_PORT", 9050u16)?,
                control_port: env_parse("NOXTERM_ANYONE_CONTROL_PORT", 9051u16)?,
                auto_start: env_parse("NOXTERM_ANYONE_AUTO_START", false)?,
            },
        })
    }

    pub fn session_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.session.idle_timeout_secs)
    }

    pub fn session_max_lifetime(&self) -> Duration {
        Duration::from_secs(self.session.max_lifetime_secs)
    }

    pub fn is_production(&self) -> bool {
        self.server.environment == Environment::Production
    }

    pub fn is_development(&self) -> bool {
        self.server.environment == Environment::Development
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::from_env().unwrap_or_else(|_| panic!("Failed to load default configuration"))
    }
}

pub fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn env_parse<T>(key: &str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(value) => value.parse().map_err(|e| ConfigError::ParseError {
            key: key.to_string(),
            message: format!("{}", e),
        }),
        Err(_) => Ok(default),
    }
}

pub fn env_list(key: &str, default: Vec<String>) -> Vec<String> {
    match env::var(key) {
        Ok(value) => value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => default,
    }
}
