// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//! Database Cleanup Operations

use super::pool::DbPool;
use tracing::info;

pub async fn run_all(pool: &DbPool) -> Result<CleanupStats, sqlx::Error> {
    let expired_sessions = cleanup_expired_sessions(pool).await?;
    let old_rate_limits = cleanup_old_rate_limits(pool).await?;
    let old_metrics = cleanup_old_metrics(pool).await?;
    let old_audit_logs = cleanup_old_audit_logs(pool).await?;

    let stats = CleanupStats {
        expired_sessions,
        old_rate_limits,
        old_metrics,
        old_audit_logs,
    };

    if stats.total() > 0 {
        info!(
            "Cleanup completed: {} expired sessions, {} rate limits, {} metrics, {} audit logs",
            expired_sessions, old_rate_limits, old_metrics, old_audit_logs
        );
    }

    Ok(stats)
}

async fn cleanup_expired_sessions(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'terminated'
        WHERE status = 'disconnected'
        AND expires_at IS NOT NULL
        AND expires_at < NOW()
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}

async fn cleanup_old_rate_limits(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM rate_limits
        WHERE window_start < NOW() - INTERVAL '1 hour'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}

async fn cleanup_old_metrics(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM container_metrics
        WHERE recorded_at < NOW() - INTERVAL '24 hours'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}

async fn cleanup_old_audit_logs(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM audit_logs
        WHERE created_at < NOW() - INTERVAL '30 days'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}

#[derive(Debug, Clone)]
pub struct CleanupStats {
    pub expired_sessions: i64,
    pub old_rate_limits: i64,
    pub old_metrics: i64,
    pub old_audit_logs: i64,
}

impl CleanupStats {
    pub fn total(&self) -> i64 {
        self.expired_sessions + self.old_rate_limits + self.old_metrics + self.old_audit_logs
    }
}
