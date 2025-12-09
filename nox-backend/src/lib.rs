// NOXTERM Library
// This file enables the backend to be used as a library

pub mod anyone_service;

pub use anyone_service::{AnyoneService, ServiceStatus};

// Re-export commonly used types
pub use anyhow::{Result, Context};
pub use uuid::Uuid;
pub use tracing::{info, warn, error, debug};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIME: &str = include_str!(concat!(env!("OUT_DIR"), "/build_time.txt"));
