// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 
//
//! Distributed rate limiting using PostgreSQL.

use super::pool::DbPool;
use tracing::debug;

pub async fn check_and_increment(
    pool: &DbPool,
    identifier: &str,
    endpoint: &str,
    max_requests: i32,
    window_seconds: i64,
) -> Result<bool, sqlx::Error> {
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

pub async fn reset(
    pool: &DbPool,
    identifier: &str,
    endpoint: Option<&str>,
) -> Result<(), sqlx::Error> {
    if let Some(ep) = endpoint {
        sqlx::query("DELETE FROM rate_limits WHERE identifier = $1 AND endpoint = $2")
            .bind(identifier)
            .bind(ep)
            .execute(pool)
            .await?;
    } else {
        sqlx::query("DELETE FROM rate_limits WHERE identifier = $1")
            .bind(identifier)
            .execute(pool)
            .await?;
    }
    Ok(())
}
