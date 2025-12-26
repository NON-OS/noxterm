// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! Security Events Database Operations

use super::pool::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SecurityEvent {
    pub id: i64,
    pub session_id: Option<Uuid>,
    pub user_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: Option<String>,
    pub blocked_input: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn log_event(
    pool: &DbPool,
    session_id: Option<Uuid>,
    user_id: &str,
    event_type: &str,
    severity: Severity,
    description: Option<&str>,
    blocked_input: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO security_events
        (session_id, user_id, event_type, severity, description, blocked_input, ip_address)
        VALUES ($1, $2, $3, $4, $5, $6, $7::INET)
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(event_type)
    .bind(severity.to_string())
    .bind(description)
    .bind(blocked_input)
    .bind(ip_address)
    .execute(pool)
    .await?;

    warn!(
        "Security event logged: {} ({}) for user {}",
        event_type, severity, user_id
    );
    Ok(())
}

pub async fn get_recent(pool: &DbPool, limit: i64) -> Result<Vec<SecurityEvent>, sqlx::Error> {
    sqlx::query_as::<_, SecurityEvent>(
        r#"
        SELECT id, session_id, user_id, event_type, severity, description,
               blocked_input, ip_address::TEXT as ip_address, created_at
        FROM security_events
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_by_severity(
    pool: &DbPool,
    severity: Severity,
    limit: i64,
) -> Result<Vec<SecurityEvent>, sqlx::Error> {
    sqlx::query_as::<_, SecurityEvent>(
        r#"
        SELECT id, session_id, user_id, event_type, severity, description,
               blocked_input, ip_address::TEXT as ip_address, created_at
        FROM security_events
        WHERE severity = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(severity.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_by_ip(
    pool: &DbPool,
    ip_address: &str,
    limit: i64,
) -> Result<Vec<SecurityEvent>, sqlx::Error> {
    sqlx::query_as::<_, SecurityEvent>(
        r#"
        SELECT id, session_id, user_id, event_type, severity, description,
               blocked_input, ip_address::TEXT as ip_address, created_at
        FROM security_events
        WHERE ip_address = $1::INET
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(ip_address)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// for threat detection
pub async fn count_by_ip(
    pool: &DbPool,
    ip_address: &str,
    window_minutes: i64,
) -> Result<i64, sqlx::Error> {
    let result: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM security_events
        WHERE ip_address = $1::INET
        AND created_at > NOW() - ($2 || ' minutes')::INTERVAL
        "#,
    )
    .bind(ip_address)
    .bind(window_minutes.to_string())
    .fetch_one(pool)
    .await?;

    Ok(result.0)
}
