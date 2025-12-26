// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! Configuration validation

use tracing::warn;

use super::error::ConfigError;
use super::types::{Config, Environment};

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::InvalidValue {
                key: "NOXTERM_PORT".to_string(),
                value: "0".to_string(),
                reason: "Port cannot be 0".to_string(),
            });
        }

        if self.docker.memory_limit_bytes < 64 * 1024 * 1024 {
            return Err(ConfigError::InvalidValue {
                key: "NOXTERM_DOCKER_MEMORY_LIMIT".to_string(),
                value: self.docker.memory_limit_bytes.to_string(),
                reason: "Memory limit must be at least 64MB".to_string(),
            });
        }

        if self.session.max_sessions_per_ip == 0 {
            return Err(ConfigError::InvalidValue {
                key: "NOXTERM_MAX_SESSIONS_PER_IP".to_string(),
                value: "0".to_string(),
                reason: "Max sessions per IP cannot be 0".to_string(),
            });
        }

        if self.docker.allowed_images.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "NOXTERM_DOCKER_ALLOWED_IMAGES".to_string(),
                value: "[]".to_string(),
                reason: "At least one container image must be allowed".to_string(),
            });
        }

        if self.server.environment == Environment::Production {
            if !self.security.validate_commands {
                warn!("Command validation is disabled in production!");
            }
            if !self.rate_limit.enabled {
                warn!("Rate limiting is disabled in production!");
            }
            if self.docker.allow_networking {
                warn!("Container networking is enabled in production - ensure this is intended");
            }
            if !self.docker.read_only_rootfs {
                warn!("Read-only root filesystem is disabled in production");
            }
            if !self.security.audit_logging {
                warn!("Audit logging is disabled in production");
            }
        }

        Ok(())
    }
}
