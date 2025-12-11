//! NOXTERM Database Layer
//!
//! PostgreSQL-backed persistent storage for sessions, audit logs, and metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
// IpAddr not currently used but may be needed for IP address parsing in future
#[allow(unused_imports)]
use std::net::IpAddr;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Database connection pool
pub type DbPool = PgPool;

/// Session status enum
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
            memory_mb: 1024,
            cpu_percent: 100,
            pids_limit: 200,
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

/// Container metrics entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContainerMetrics {
    pub id: i64,
    pub session_id: Uuid,
    pub cpu_percent: Option<f64>,
    pub memory_usage: Option<i64>,
    pub memory_limit: Option<i64>,
    pub network_rx: Option<i64>,
    pub network_tx: Option<i64>,
    pub recorded_at: DateTime<Utc>,
}

/// Security event entry
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

/// Rate limit entry
#[derive(Debug, Clone, FromRow)]
pub struct RateLimitEntry {
    pub id: i64,
    pub identifier: String,
    pub endpoint: String,
    pub request_count: i32,
    pub window_start: DateTime<Utc>,
}

/// Initialize database connection pool
pub async fn init_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    info!("Connecting to PostgreSQL database...");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await?;

    info!("PostgreSQL connection pool established");
    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::Error> {
    info!("Running database migrations...");

    // Read migration file
    let migration_sql = include_str!("../migrations/001_initial.sql");

    // Execute migration
    sqlx::raw_sql(migration_sql).execute(pool).await?;

    info!("Database migrations completed successfully");
    Ok(())
}

/// Database operations for sessions
pub mod sessions {
    use super::*;

    /// Create a new session
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

    /// Get session by ID
    pub async fn get_by_id(pool: &DbPool, id: Uuid) -> Result<Option<DbSession>, sqlx::Error> {
        let session = sqlx::query_as::<_, DbSession>(
            "SELECT * FROM sessions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Get all sessions for a user
    pub async fn get_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<DbSession>, sqlx::Error> {
        let sessions = sqlx::query_as::<_, DbSession>(
            r#"
            SELECT * FROM sessions
            WHERE user_id = $1
            AND status NOT IN ('terminated')
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Get active sessions for a user (not disconnected or terminated)
    pub async fn get_active_by_user(pool: &DbPool, user_id: &str) -> Result<Vec<DbSession>, sqlx::Error> {
        let sessions = sqlx::query_as::<_, DbSession>(
            r#"
            SELECT * FROM sessions
            WHERE user_id = $1
            AND status IN ('created', 'running')
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Count active containers for a user
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

    /// Update session status
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

    /// Update session with container info
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

    /// Mark session as disconnected with expiration
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

        debug!("Marked session {} as disconnected, expires in {} seconds", id, grace_period_secs);
        Ok(())
    }

    /// Clear disconnection status (for reattach)
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

    /// Mark session as terminated
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

    /// Get expired sessions for cleanup
    pub async fn get_expired(pool: &DbPool) -> Result<Vec<DbSession>, sqlx::Error> {
        let sessions = sqlx::query_as::<_, DbSession>(
            r#"
            SELECT * FROM sessions
            WHERE status = 'disconnected'
            AND expires_at IS NOT NULL
            AND expires_at < NOW()
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Update last activity timestamp
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
        let sessions = match (user_id, status) {
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
                .await?
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
                .await?
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
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, DbSession>(
                    "SELECT * FROM sessions ORDER BY created_at DESC LIMIT $1",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(sessions)
    }
}

/// Database operations for audit logs
pub mod audit {
    use super::*;

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
        let logs = sqlx::query_as::<_, AuditLog>(
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
        .await?;

        Ok(logs)
    }

    /// Get audit logs for a user
    pub async fn get_by_user(
        pool: &DbPool,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, AuditLog>(
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
        .await?;

        Ok(logs)
    }
}

/// Database operations for security events
pub mod security {
    use super::*;

    /// Security event severity levels
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

    /// Log a security event
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

    /// Get recent security events
    pub async fn get_recent(
        pool: &DbPool,
        limit: i64,
    ) -> Result<Vec<SecurityEvent>, sqlx::Error> {
        let events = sqlx::query_as::<_, SecurityEvent>(
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
        .await?;

        Ok(events)
    }
}

/// Database operations for container metrics
pub mod metrics {
    use super::*;

    /// Record container metrics
    pub async fn record(
        pool: &DbPool,
        session_id: Uuid,
        cpu_percent: Option<f64>,
        memory_usage: Option<i64>,
        memory_limit: Option<i64>,
        network_rx: Option<i64>,
        network_tx: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO container_metrics
            (session_id, cpu_percent, memory_usage, memory_limit, network_rx, network_tx)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(session_id)
        .bind(cpu_percent)
        .bind(memory_usage)
        .bind(memory_limit)
        .bind(network_rx)
        .bind(network_tx)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get latest metrics for a session
    pub async fn get_latest(
        pool: &DbPool,
        session_id: Uuid,
    ) -> Result<Option<ContainerMetrics>, sqlx::Error> {
        let metrics = sqlx::query_as::<_, ContainerMetrics>(
            r#"
            SELECT * FROM container_metrics
            WHERE session_id = $1
            ORDER BY recorded_at DESC
            LIMIT 1
            "#,
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await?;

        Ok(metrics)
    }

    /// Get metrics history for a session
    pub async fn get_history(
        pool: &DbPool,
        session_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ContainerMetrics>, sqlx::Error> {
        let metrics = sqlx::query_as::<_, ContainerMetrics>(
            r#"
            SELECT * FROM container_metrics
            WHERE session_id = $1
            ORDER BY recorded_at DESC
            LIMIT $2
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(metrics)
    }
}

/// Database operations for rate limiting
pub mod rate_limits {
    use super::*;

    /// Check and increment rate limit
    /// Returns true if request is allowed, false if rate limited
    pub async fn check_and_increment(
        pool: &DbPool,
        identifier: &str,
        endpoint: &str,
        max_requests: i32,
        window_seconds: i64,
    ) -> Result<bool, sqlx::Error> {
        // Use a transaction to ensure atomicity
        let result: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO rate_limits (identifier, endpoint, request_count, window_start)
            VALUES ($1, $2, 1, date_trunc('minute', NOW()))
            ON CONFLICT (identifier, endpoint, window_start)
            DO UPDATE SET request_count = rate_limits.request_count + 1
            WHERE rate_limits.window_start > NOW() - ($3 || ' seconds')::INTERVAL
            RETURNING request_count
            "#,
        )
        .bind(identifier)
        .bind(endpoint)
        .bind(window_seconds.to_string())
        .fetch_one(pool)
        .await?;

        let allowed = result.0 <= max_requests;

        if !allowed {
            debug!(
                "Rate limit exceeded for {} on {}: {} requests",
                identifier, endpoint, result.0
            );
        }

        Ok(allowed)
    }

    /// Get current request count for an identifier
    pub async fn get_count(
        pool: &DbPool,
        identifier: &str,
        endpoint: &str,
        window_seconds: i64,
    ) -> Result<i32, sqlx::Error> {
        let result: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT SUM(request_count)::INT
            FROM rate_limits
            WHERE identifier = $1
            AND endpoint = $2
            AND window_start > NOW() - ($3 || ' seconds')::INTERVAL
            "#,
        )
        .bind(identifier)
        .bind(endpoint)
        .bind(window_seconds.to_string())
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.0).unwrap_or(0))
    }
}

/// Cleanup operations
pub mod cleanup {
    use super::*;

    /// Run all cleanup operations
    pub async fn run_all(pool: &DbPool) -> Result<(), sqlx::Error> {
        // Clean expired sessions
        let expired_count: (i32,) =
            sqlx::query_as("SELECT cleanup_expired_sessions()")
                .fetch_one(pool)
                .await?;

        if expired_count.0 > 0 {
            info!("Cleaned up {} expired sessions", expired_count.0);
        }

        // Clean old rate limits
        let rate_count: (i32,) =
            sqlx::query_as("SELECT cleanup_old_rate_limits()")
                .fetch_one(pool)
                .await?;

        if rate_count.0 > 0 {
            debug!("Cleaned up {} old rate limit entries", rate_count.0);
        }

        // Clean old metrics
        let metrics_count: (i32,) =
            sqlx::query_as("SELECT cleanup_old_metrics()")
                .fetch_one(pool)
                .await?;

        if metrics_count.0 > 0 {
            debug!("Cleaned up {} old metrics entries", metrics_count.0);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Created.to_string(), "created");
        assert_eq!(SessionStatus::Running.to_string(), "running");
        assert_eq!(SessionStatus::Disconnected.to_string(), "disconnected");
        assert_eq!(SessionStatus::Terminated.to_string(), "terminated");
    }

    #[test]
    fn test_session_status_from_str() {
        assert_eq!(SessionStatus::from("created"), SessionStatus::Created);
        assert_eq!(SessionStatus::from("RUNNING"), SessionStatus::Running);
        assert_eq!(SessionStatus::from("unknown"), SessionStatus::Created);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_mb, 1024);
        assert_eq!(limits.cpu_percent, 100);
        assert_eq!(limits.pids_limit, 200);
    }
}
