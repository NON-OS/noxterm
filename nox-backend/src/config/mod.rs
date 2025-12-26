// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! NOXTERM Configuration Module
//! All configuration values are loaded from NOXTERM_* environment variables.


mod error;
mod loader;
mod types;
mod validation;

pub use error::ConfigError;
pub use loader::{env_list, env_or, env_parse};
pub use types::{
    AnyoneConfig, Config, DatabaseConfig, DockerConfig, Environment, ObservabilityConfig,
    RateLimitConfig, SecurityConfig, ServerConfig, SessionConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_parsing() {
        assert_eq!(
            "production".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "prod".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "development".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "dev".parse::<Environment>().unwrap(),
            Environment::Development
        );
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Production.to_string(), "production");
        assert_eq!(Environment::Development.to_string(), "development");
    }
}
