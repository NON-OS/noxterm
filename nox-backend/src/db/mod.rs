// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! NOXTERM Database-Layer
//! PostgreSQL-backed persistent storage for sessions, audit logs and metrics.

pub mod audit;
pub mod cleanup;
pub mod metrics;
mod pool;
pub mod rate_limits;
pub mod security;
pub mod sessions;

pub use audit::{AuditLog, EventType};
pub use metrics::ContainerMetrics;
pub use pool::{init_pool, run_migrations, DbPool};
pub use security::SecurityEvent;
pub use sessions::{DbSession, ResourceLimits, SessionStatus};
