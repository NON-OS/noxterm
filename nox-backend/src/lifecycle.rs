//! NOXTERM Container Lifecycle Management
//!
//! Background tasks for container cleanup, health monitoring, and session management.

use crate::db::{self, DbPool};
use bollard::container::{InspectContainerOptions, StatsOptions, StopContainerOptions};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for lifecycle management
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Grace period in seconds before cleaning up disconnected sessions
    pub grace_period_secs: i64,
    /// Interval for cleanup task in seconds
    pub cleanup_interval_secs: u64,
    /// Interval for health check task in seconds
    pub health_check_interval_secs: u64,
    /// Interval for metrics collection in seconds
    pub metrics_interval_secs: u64,
    /// Maximum containers per user
    pub max_containers_per_user: i64,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            grace_period_secs: 300, // 5 minutes
            cleanup_interval_secs: 60,
            health_check_interval_secs: 30,
            metrics_interval_secs: 15,
            max_containers_per_user: 3,
        }
    }
}

/// Container health status
#[derive(Debug, Clone)]
pub struct ContainerHealth {
    pub container_id: String,
    pub session_id: Uuid,
    pub is_running: bool,
    pub cpu_percent: Option<f64>,
    pub memory_usage: Option<i64>,
    pub memory_limit: Option<i64>,
    pub network_rx: Option<i64>,
    pub network_tx: Option<i64>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Lifecycle manager for handling background tasks
pub struct LifecycleManager {
    docker: Docker,
    db_pool: DbPool,
    config: LifecycleConfig,
    /// Cache of active container health statuses
    health_cache: Arc<RwLock<HashMap<Uuid, ContainerHealth>>>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(docker: Docker, db_pool: DbPool, config: LifecycleConfig) -> Self {
        Self {
            docker,
            db_pool,
            config,
            health_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start all background tasks
    pub async fn start(self: Arc<Self>) {
        info!("Starting lifecycle management background tasks");

        let cleanup_manager = self.clone();
        let health_manager = self.clone();
        let metrics_manager = self.clone();
        let orphan_manager = self.clone();

        // Spawn cleanup task
        tokio::spawn(async move {
            cleanup_manager.run_cleanup_task().await;
        });

        // Spawn health check task
        tokio::spawn(async move {
            health_manager.run_health_check_task().await;
        });

        // Spawn metrics collection task
        tokio::spawn(async move {
            metrics_manager.run_metrics_task().await;
        });

        // Spawn orphan container detection task
        tokio::spawn(async move {
            orphan_manager.run_orphan_detection_task().await;
        });

        info!("Lifecycle management tasks started");
    }

    /// Cleanup task - removes expired sessions and containers
    async fn run_cleanup_task(&self) {
        let mut ticker = interval(Duration::from_secs(self.config.cleanup_interval_secs));

        loop {
            ticker.tick().await;
            debug!("Running cleanup task");

            // Get expired sessions
            match db::sessions::get_expired(&self.db_pool).await {
                Ok(expired_sessions) => {
                    for session in expired_sessions {
                        info!(
                            "Cleaning up expired session {} (user: {})",
                            session.id, session.user_id
                        );

                        // Stop and remove container if exists
                        if let Some(container_id) = &session.container_id {
                            if let Err(e) = self.stop_container(container_id).await {
                                warn!("Failed to stop container {}: {}", container_id, e);
                            }
                        }

                        // Mark session as terminated
                        if let Err(e) = db::sessions::terminate(&self.db_pool, session.id).await {
                            error!("Failed to terminate session {}: {}", session.id, e);
                        }

                        // Log audit event
                        let _ = db::audit::log(
                            &self.db_pool,
                            Some(session.id),
                            &session.user_id,
                            db::audit::EventType::SessionTerminated,
                            Some(serde_json::json!({
                                "reason": "grace_period_expired"
                            })),
                            None,
                            None,
                        )
                        .await;

                        // Remove from health cache
                        self.health_cache.write().await.remove(&session.id);
                    }
                }
                Err(e) => {
                    error!("Failed to get expired sessions: {}", e);
                }
            }

            // Run database cleanup
            if let Err(e) = db::cleanup::run_all(&self.db_pool).await {
                error!("Database cleanup failed: {}", e);
            }
        }
    }

    /// Health check task - monitors container status
    async fn run_health_check_task(&self) {
        let mut ticker = interval(Duration::from_secs(self.config.health_check_interval_secs));

        loop {
            ticker.tick().await;
            debug!("Running health check task");

            // Get all running sessions
            match db::sessions::list(&self.db_pool, None, Some("running"), 1000).await {
                Ok(sessions) => {
                    for session in sessions {
                        if let Some(container_id) = &session.container_id {
                            match self.check_container_health(container_id, session.id).await {
                                Ok(health) => {
                                    // Update health cache
                                    self.health_cache.write().await.insert(session.id, health);
                                }
                                Err(e) => {
                                    warn!(
                                        "Health check failed for container {}: {}",
                                        container_id, e
                                    );

                                    // Container might have crashed - check if it exists
                                    if let Ok(false) = self.container_exists(container_id).await {
                                        warn!(
                                            "Container {} no longer exists, marking session {} as disconnected",
                                            container_id, session.id
                                        );

                                        // Mark as disconnected with grace period
                                        let _ = db::sessions::mark_disconnected(
                                            &self.db_pool,
                                            session.id,
                                            self.config.grace_period_secs,
                                        )
                                        .await;

                                        // Log container stopped event
                                        let _ = db::audit::log(
                                            &self.db_pool,
                                            Some(session.id),
                                            &session.user_id,
                                            db::audit::EventType::ContainerStopped,
                                            Some(serde_json::json!({
                                                "reason": "container_crashed"
                                            })),
                                            None,
                                            None,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get running sessions: {}", e);
                }
            }
        }
    }

    /// Metrics collection task - records container resource usage
    async fn run_metrics_task(&self) {
        let mut ticker = interval(Duration::from_secs(self.config.metrics_interval_secs));

        loop {
            ticker.tick().await;
            debug!("Running metrics collection task");

            // Get health data from cache and record metrics
            let health_data: Vec<ContainerHealth> =
                self.health_cache.read().await.values().cloned().collect();

            for health in health_data {
                if let Err(e) = db::metrics::record(
                    &self.db_pool,
                    health.session_id,
                    health.cpu_percent,
                    health.memory_usage,
                    health.memory_limit,
                    health.network_rx,
                    health.network_tx,
                )
                .await
                {
                    debug!("Failed to record metrics for session {}: {}", health.session_id, e);
                }
            }
        }
    }

    /// Orphan container detection - finds and removes containers not tracked in DB
    async fn run_orphan_detection_task(&self) {
        // Run less frequently
        let mut ticker = interval(Duration::from_secs(300)); // Every 5 minutes

        loop {
            ticker.tick().await;
            debug!("Running orphan container detection");

            // List all noxterm containers
            match self.list_noxterm_containers().await {
                Ok(container_ids) => {
                    for container_id in container_ids {
                        // Check if this container is tracked in DB
                        let is_tracked = self.is_container_tracked(&container_id).await;

                        if !is_tracked {
                            warn!(
                                "Found orphan container {}, scheduling for removal",
                                container_id
                            );

                            // Stop and remove orphan container
                            if let Err(e) = self.stop_container(&container_id).await {
                                error!("Failed to remove orphan container {}: {}", container_id, e);
                            } else {
                                info!("Removed orphan container {}", container_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to list containers: {}", e);
                }
            }
        }
    }

    /// Check health of a specific container
    async fn check_container_health(
        &self,
        container_id: &str,
        session_id: Uuid,
    ) -> Result<ContainerHealth, anyhow::Error> {
        // Get container stats
        let mut stats_stream = self.docker.stats(
            container_id,
            Some(StatsOptions {
                stream: false,
                ..Default::default()
            }),
        );

        if let Some(stats_result) = stats_stream.next().await {
            let stats = stats_result?;

            // Calculate CPU percentage
            // Note: bollard 0.17 has total_usage as u64 directly, not Option<u64>
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage
                .saturating_sub(stats.precpu_stats.cpu_usage.total_usage) as f64;
            let system_delta = stats
                .cpu_stats
                .system_cpu_usage
                .unwrap_or(0)
                .saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0))
                as f64;

            let cpu_percent = if system_delta > 0.0 {
                let num_cpus = stats
                    .cpu_stats
                    .online_cpus
                    .unwrap_or(1) as f64;
                Some((cpu_delta / system_delta) * num_cpus * 100.0)
            } else {
                Some(0.0)
            };

            // Get memory stats
            let memory_usage = stats.memory_stats.usage.map(|u| u as i64);
            let memory_limit = stats.memory_stats.limit.map(|l| l as i64);

            // Get network stats
            let (network_rx, network_tx) = if let Some(networks) = &stats.networks {
                let mut rx: i64 = 0;
                let mut tx: i64 = 0;
                for (_, net_stats) in networks {
                    rx += net_stats.rx_bytes as i64;
                    tx += net_stats.tx_bytes as i64;
                }
                (Some(rx), Some(tx))
            } else {
                (None, None)
            };

            Ok(ContainerHealth {
                container_id: container_id.to_string(),
                session_id,
                is_running: true,
                cpu_percent,
                memory_usage,
                memory_limit,
                network_rx,
                network_tx,
                last_check: chrono::Utc::now(),
            })
        } else {
            anyhow::bail!("No stats available for container")
        }
    }

    /// Check if container exists
    async fn container_exists(&self, container_id: &str) -> Result<bool, anyhow::Error> {
        match self
            .docker
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
        {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Stop and remove a container gracefully
    pub async fn stop_container(&self, container_id: &str) -> Result<(), anyhow::Error> {
        // Try graceful stop first (SIGTERM)
        let stop_result = self
            .docker
            .stop_container(
                container_id,
                Some(StopContainerOptions { t: 10 }), // 10 second timeout
            )
            .await;

        match stop_result {
            Ok(_) => {
                debug!("Container {} stopped gracefully", container_id);
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                debug!("Container {} already removed", container_id);
                return Ok(());
            }
            Err(e) => {
                warn!("Graceful stop failed for {}, forcing: {}", container_id, e);
                // Force kill
                let _ = self.docker.kill_container::<String>(container_id, None).await;
            }
        }

        // Remove container
        match self
            .docker
            .remove_container(
                container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
        {
            Ok(_) => {
                debug!("Container {} removed", container_id);
                Ok(())
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                debug!("Container {} already removed", container_id);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// List all noxterm session containers (not infrastructure like postgres)
    async fn list_noxterm_containers(&self) -> Result<Vec<String>, anyhow::Error> {
        use bollard::container::ListContainersOptions;
        use std::collections::HashMap;

        let mut filters = HashMap::new();
        // Only target session containers (noxterm-session-*), not infrastructure (noxterm-postgres)
        filters.insert("name", vec!["noxterm-session-"]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await?;

        Ok(containers
            .iter()
            .filter_map(|c| c.id.clone())
            .collect())
    }

    /// Check if a container is tracked in the database
    async fn is_container_tracked(&self, container_id: &str) -> bool {
        // Query database for this container ID
        let result: Result<Option<(i64,)>, _> = sqlx::query_as(
            "SELECT 1 FROM sessions WHERE container_id = $1 AND status != 'terminated' LIMIT 1",
        )
        .bind(container_id)
        .fetch_optional(&self.db_pool)
        .await;

        match result {
            Ok(Some(_)) => true,
            _ => false,
        }
    }

    /// Get cached health status for a session
    pub async fn get_health(&self, session_id: Uuid) -> Option<ContainerHealth> {
        self.health_cache.read().await.get(&session_id).cloned()
    }

    /// Get all cached health statuses
    pub async fn get_all_health(&self) -> Vec<ContainerHealth> {
        self.health_cache.read().await.values().cloned().collect()
    }

    /// Remove session from health cache
    pub async fn remove_from_cache(&self, session_id: Uuid) {
        self.health_cache.write().await.remove(&session_id);
    }

    /// Check if user can create more containers
    pub async fn can_create_container(&self, user_id: &str) -> Result<bool, anyhow::Error> {
        let count = db::sessions::count_active_by_user(&self.db_pool, user_id).await?;
        Ok(count < self.config.max_containers_per_user)
    }

    /// Get user's container count
    pub async fn get_user_container_count(&self, user_id: &str) -> Result<i64, anyhow::Error> {
        Ok(db::sessions::count_active_by_user(&self.db_pool, user_id).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.grace_period_secs, 300);
        assert_eq!(config.max_containers_per_user, 3);
    }
}
