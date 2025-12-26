// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//! Audit Log Database Operations

use super::pool::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use tracing::debug;
use uuid::Uuid;

/// Event types for audit logging
#[derive(Debug, Clone, Serialize)]
pub enum EventType {
    SessionCreated,
    SessionConnected,
    SessionDisconnected,
    SessionTerminated,
    ContainerStarted,
    ContainerStopped,
    CommandExecuted,
    SecurityViolation,
    RateLimitExceeded,
    AuthAttempt,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::SessionCreated => write!(f, "session_created"),
            EventType::SessionConnected => write!(f, "session_connected"),
            EventType::SessionDisconnected => write!(f, "session_disconnected"),
            EventType::SessionTerminated => write!(f, "session_terminated"),
            EventType::ContainerStarted => write!(f, "container_started"),
            EventType::ContainerStopped => write!(f, "container_stopped"),
            EventType::CommandExecuted => write!(f, "command_executed"),
            EventType::SecurityViolation => write!(f, "security_violation"),
            EventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            EventType::AuthAttempt => write!(f, "auth_attempt"),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub session_id: Option<Uuid>,
    pub user_id: String,
    pub event_type: String,
    pub event_data: Option<JsonValue>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Log an audit event
pub async fn log(
    pool: &DbPool,
    session_id: Option<Uuid>,
    user_id: &str,
    event_type: EventType,
    event_data: Option<JsonValue>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (session_id, user_id, event_type, event_data, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5::INET, $6)
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(event_type.to_string())
    .bind(event_data)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    debug!("Logged audit event: {} for user {}", event_type, user_id);
    Ok(())
}

/// Get audit logs for a session
pub async fn get_by_session(
    pool: &DbPool,
    session_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, session_id, user_id, event_type, event_data,
               ip_address::TEXT as ip_address, user_agent, created_at
        FROM audit_logs
        WHERE session_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get audit logs for a user
pub async fn get_by_user(
    pool: &DbPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, session_id, user_id, event_type, event_data,
               ip_address::TEXT as ip_address, user_agent, created_at
        FROM audit_logs
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get recent audit logs
pub async fn get_recent(pool: &DbPool, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"
        SELECT id, session_id, user_id, event_type, event_data,
               ip_address::TEXT as ip_address, user_agent, created_at
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}
