// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//! Configuration error types

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {key}")]
    MissingRequired { key: String },

    #[error("Invalid value for {key}: '{value}' - {reason}")]
    InvalidValue {
        key: String,
        value: String,
        reason: String,
    },

    #[error("Parse error for {key}: {message}")]
    ParseError { key: String, message: String },
}
