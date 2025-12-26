// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//! Docker/Container error types

use std::fmt;
#[derive(Debug)]
pub enum DockerError {
    ConnectionFailed(String),
    ContainerCreateFailed(String),
    ContainerStartFailed(String),
    ContainerStopFailed(String),
    ExecFailed(String),
    ImageNotAllowed(String),
    ImagePullFailed(String),
    ResourceLimitExceeded(String),
}

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockerError::ConnectionFailed(msg) => write!(f, "Docker connection failed: {}", msg),
            DockerError::ContainerCreateFailed(msg) => {
                write!(f, "Container creation failed: {}", msg)
            }
            DockerError::ContainerStartFailed(msg) => write!(f, "Container start failed: {}", msg),
            DockerError::ContainerStopFailed(msg) => write!(f, "Container stop failed: {}", msg),
            DockerError::ExecFailed(msg) => write!(f, "Exec failed: {}", msg),
            DockerError::ImageNotAllowed(img) => write!(f, "Image not allowed: {}", img),
            DockerError::ImagePullFailed(msg) => write!(f, "Image pull failed: {}", msg),
            DockerError::ResourceLimitExceeded(msg) => {
                write!(f, "Resource limit exceeded: {}", msg)
            }
        }
    }
}
