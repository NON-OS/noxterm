// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! CRUD operations for session persistence.

use super::pool::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use tracing::debug;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Created,
    Running,
    Disconnected,
    Terminated,
}

impl From<&str> for SessionStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "created" => SessionStatus::Created,
            "running" => SessionStatus::Running,
            "disconnected" => SessionStatus::Disconnected,
            "terminated" => SessionStatus::Terminated,
            _ => SessionStatus::Created,
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Created => write!(f, "created"),
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Disconnected => write!(f, "disconnected"),
            SessionStatus::Terminated => write!(f, "terminated"),
        }
    }
}

/// Resource limits for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: i64,
    pub cpu_percent: i64,
    pub pids_limit: i64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 512,
            cpu_percent: 50,
            pids_limit: 100,
        }
    }
}

/// Persistent session model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbSession {
    pub id: Uuid,
    pub user_id: String,
    pub status: String,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub container_image: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub resource_limits: JsonValue,
    pub metadata: JsonValue,
}

pub async fn create(
    pool: &DbPool,
    id: Uuid,
    user_id: &str,
    container_image: &str,
    resource_limits: Option<ResourceLimits>,
) -> Result<DbSession, sqlx::Error> {
    let limits = resource_limits.unwrap_or_default();
    let limits_json = serde_json::to_value(&limits).unwrap_or_default();

    let session = sqlx::query_as::<_, DbSession>(
        r#"
        INSERT INTO sessions (id, user_id, container_image, resource_limits)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(container_image)
    .bind(limits_json)
    .fetch_one(pool)
    .await?;

    debug!("Created session {} for user {}", id, user_id);
    Ok(session)
}

pub async fn get_by_id(pool: &DbPool, id: Uuid) -> Result<Option<DbSession>, sqlx::Error> {
    sqlx::query_as::<_, DbSession>("SELECT * FROM sessions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<DbSession>, sqlx::Error> {
    sqlx::query_as::<_, DbSession>(
        r#"
        SELECT * FROM sessions
        WHERE user_id = $1
        AND status NOT IN ('terminated')
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_active_by_user(
    pool: &DbPool,
    user_id: &str,
) -> Result<Vec<DbSession>, sqlx::Error> {
    sqlx::query_as::<_, DbSession>(
        r#"
        SELECT * FROM sessions
        WHERE user_id = $1
        AND status IN ('created', 'running')
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn count_active_by_user(pool: &DbPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM sessions
        WHERE user_id = $1
        AND status IN ('created', 'running')
        AND container_id IS NOT NULL
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

pub async fn update_status(
    pool: &DbPool,
    id: Uuid,
    status: SessionStatus,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET status = $1 WHERE id = $2")
        .bind(status.to_string())
        .bind(id)
        .execute(pool)
        .await?;

    debug!("Updated session {} status to {}", id, status);
    Ok(())
}

pub async fn set_container(
    pool: &DbPool,
    id: Uuid,
    container_id: &str,
    container_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET container_id = $1, container_name = $2, status = 'running'
        WHERE id = $3
        "#,
    )
    .bind(container_id)
    .bind(container_name)
    .bind(id)
    .execute(pool)
    .await?;

    debug!("Set container {} for session {}", container_id, id);
    Ok(())
}

pub async fn mark_disconnected(
    pool: &DbPool,
    id: Uuid,
    grace_period_secs: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'disconnected',
            disconnected_at = NOW(),
            expires_at = NOW() + ($1 || ' seconds')::INTERVAL
        WHERE id = $2
        "#,
    )
    .bind(grace_period_secs.to_string())
    .bind(id)
    .execute(pool)
    .await?;

    debug!(
        "Marked session {} as disconnected, expires in {} seconds",
        id, grace_period_secs
    );
    Ok(())
}

pub async fn clear_disconnection(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'running',
            disconnected_at = NULL,
            expires_at = NULL
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    debug!("Cleared disconnection for session {}", id);
    Ok(())
}

pub async fn terminate(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'terminated', container_id = NULL, container_name = NULL
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    debug!("Terminated session {}", id);
    Ok(())
}

pub async fn get_expired(pool: &DbPool) -> Result<Vec<DbSession>, sqlx::Error> {
    sqlx::query_as::<_, DbSession>(
        r#"
        SELECT * FROM sessions
        WHERE status = 'disconnected'
        AND expires_at IS NOT NULL
        AND expires_at < NOW()
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn touch(pool: &DbPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET last_activity = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all sessions with optional filters
pub async fn list(
    pool: &DbPool,
    user_id: Option<&str>,
    status: Option<&str>,
    limit: i64,
) -> Result<Vec<DbSession>, sqlx::Error> {
    match (user_id, status) {
        (Some(uid), Some(st)) => {
            sqlx::query_as::<_, DbSession>(
                r#"
                SELECT * FROM sessions
                WHERE user_id = $1 AND status = $2
                ORDER BY created_at DESC
                LIMIT $3
                "#,
            )
            .bind(uid)
            .bind(st)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (Some(uid), None) => {
            sqlx::query_as::<_, DbSession>(
                r#"
                SELECT * FROM sessions
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(uid)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, Some(st)) => {
            sqlx::query_as::<_, DbSession>(
                r#"
                SELECT * FROM sessions
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(st)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, DbSession>(
                "SELECT * FROM sessions ORDER BY created_at DESC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Created.to_string(), "created");
        assert_eq!(SessionStatus::Running.to_string(), "running");
    }

    #[test]
    fn test_session_status_from_str() {
        assert_eq!(SessionStatus::from("created"), SessionStatus::Created);
        assert_eq!(SessionStatus::from("RUNNING"), SessionStatus::Running);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 512);
        assert_eq!(limits.pids_limit, 100);
    }
}
