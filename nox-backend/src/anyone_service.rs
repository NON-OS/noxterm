use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{sleep, Duration, timeout};
use tracing::{info, warn, debug};
use reqwest::Client;
use tokio::process::{Child, Command as TokioCommand};
use anyhow::{Result, Context};

/// NOX Rust, Anyone Protocol service manager
#[derive(Clone)]
pub struct AnyoneService {
    process: Arc<Mutex<Option<Child>>>,
    socks_port: u16,
    control_port: u16,
    enabled: Arc<RwLock<bool>>,
    status: Arc<RwLock<ServiceStatus>>,
    client: Arc<RwLock<Option<Client>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

impl AnyoneService {
    pub fn new(socks_port: u16, control_port: u16) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            socks_port,
            control_port,
            enabled: Arc::new(RwLock::new(false)),
            status: Arc::new(RwLock::new(ServiceStatus::Stopped)),
            client: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut status = self.status.write().await;
        
        if *status == ServiceStatus::Running {
            return Ok(()); // Already running
        }
        
        *status = ServiceStatus::Starting;
        drop(status);

        info!("Starting Anyone Protocol service on SOCKS port {}, Control port {}", 
              self.socks_port, self.control_port);

        // Check if Node.js and npm are available
        self.check_prerequisites().await?;

        // Install Anyone client if not present
        self.ensure_anyone_client_installed().await?;

        // Create terms agreement file
        self.create_terms_agreement().await?;

        // Start the Anyone client process
        let mut process = self.process.lock().await;
        
        if process.is_some() {
            return Ok(()); // Race condition protection
        }

        let child = self.spawn_anyone_process().await?;
        *process = Some(child);
        drop(process);

        // Wait for service to be ready with timeout
        self.wait_for_ready().await?;

        // Initialize HTTP client with SOCKS proxy
        self.initialize_proxy_client().await?;

        // Update status
        *self.enabled.write().await = true;
        *self.status.write().await = ServiceStatus::Running;

        info!("✅ Anyone Protocol service started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut status = self.status.write().await;
        
        if *status == ServiceStatus::Stopped {
            return Ok(());
        }
        
        *status = ServiceStatus::Stopping;
        drop(status);

        info!("Stopping Anyone Protocol service...");

        let mut process = self.process.lock().await;
        
        if let Some(mut child) = process.take() {
            // Graceful shutdown first
            if let Err(e) = child.kill().await {
                warn!("Failed to kill Anyone process gracefully: {}", e);
            }
            
            // Wait for termination with timeout
            match timeout(Duration::from_secs(10), child.wait()).await {
                Ok(Ok(status)) => {
                    debug!("Anyone process exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    warn!("Error waiting for Anyone process: {}", e);
                }
                Err(_) => {
                    warn!("Anyone process did not exit within timeout, force killing");
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }

        // Clear client
        *self.client.write().await = None;
        *self.enabled.write().await = false;
        *self.status.write().await = ServiceStatus::Stopped;

        info!("Anyone Protocol service stopped");
        Ok(())
    }

    /// Check if the service is currently enabled and running
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Get the current service status
    pub async fn get_status(&self) -> ServiceStatus {
        self.status.read().await.clone()
    }

    /// Get SOCKS port
    pub fn get_socks_port(&self) -> u16 {
        self.socks_port
    }

    /// Get control port
    pub fn get_control_port(&self) -> u16 {
        self.control_port
    }

    /// Get HTTP client configured for SOCKS proxy
    pub async fn get_proxy_client(&self) -> Option<Client> {
        self.client.read().await.clone()
    }

    pub async fn check_ports_available(&self) -> Result<()> {
        use std::net::{TcpListener, SocketAddr};
        
        let socks_addr: SocketAddr = format!("127.0.0.1:{}", self.socks_port).parse()?;
        let control_addr: SocketAddr = format!("127.0.0.1:{}", self.control_port).parse()?;
        
        // Check SOCKS port
        match TcpListener::bind(socks_addr) {
            Ok(_) => {},
            Err(_) => {
                return Err(anyhow::anyhow!("SOCKS port {} is already in use", self.socks_port));
            }
        }
        
        // Check control port  
        match TcpListener::bind(control_addr) {
            Ok(_) => {},
            Err(_) => {
                return Err(anyhow::anyhow!("Control port {} is already in use", self.control_port));
            }
        }
        
        Ok(())
    }

    // Private implementation methods
    
    async fn check_prerequisites(&self) -> Result<()> {
        let node_output = Command::new("node")
            .args(&["--version"])
            .output()
            .context("Failed to execute node command")?;
            
        if !node_output.status.success() {
            return Err(anyhow::anyhow!("Node.js is not installed or not accessible"));
        }
        
        let version = String::from_utf8_lossy(&node_output.stdout);
        debug!("Node.js version: {}", version.trim());

        let npm_output = Command::new("npm")
            .args(&["--version"])
            .output()
            .context("Failed to execute npm command")?;
            
        if !npm_output.status.success() {
            return Err(anyhow::anyhow!("npm is not installed or not accessible"));
        }
        
        let npm_version = String::from_utf8_lossy(&npm_output.stdout);
        debug!("npm version: {}", npm_version.trim());

        Ok(())
    }

    async fn ensure_anyone_client_installed(&self) -> Result<()> {
        let check_output = Command::new("npm")
            .args(&["list", "-g", "@anyone-protocol/anyone-client"])
            .output();

        if let Ok(output) = check_output {
            if output.status.success() {
                debug!("Anyone client already available");
                return Ok(());
            }
        }

        info!("Installing Anyone Protocol client...");
        
        let install_output = Command::new("npm")
            .args(&["install", "-g", "@anyone-protocol/anyone-client"])
            .output()
            .context("Failed to execute npm install")?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            return Err(anyhow::anyhow!("Failed to install Anyone client: {}", stderr));
        }

        info!("Successfully installed Anyone Protocol client");
        Ok(())
    }

    async fn create_terms_agreement(&self) -> Result<()> {
        tokio::fs::write("terms-agreement", "agreed")
            .await
            .context("Failed to create terms-agreement file")?;
        debug!("Created terms-agreement file");
        Ok(())
    }

    async fn spawn_anyone_process(&self) -> Result<Child> {
        let child = TokioCommand::new("npx")
            .args(&[
                "@anyone-protocol/anyone-client",
                "-s", &self.socks_port.to_string(),
                "-c", &self.control_port.to_string(),
                "-v"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn Anyone client process")?;

        debug!("Spawned Anyone client process with PID: {:?}", child.id());
        Ok(child)
    }

    async fn wait_for_ready(&self) -> Result<()> {
        let max_attempts = 30; // 30 seconds max
        
        for attempt in 1..=max_attempts {
            debug!("Checking if Anyone service is ready, attempt {}/{}", attempt, max_attempts);
            
            if self.check_socks_connectivity().await.is_ok() {
                info!("Anyone Protocol service is ready");
                return Ok(());
            }
            
            sleep(Duration::from_secs(1)).await;
        }
        
        *self.status.write().await = ServiceStatus::Error("Service failed to start within timeout".to_string());
        Err(anyhow::anyhow!("Anyone Protocol service failed to start within timeout"))
    }

    async fn check_socks_connectivity(&self) -> Result<()> {
        use std::net::TcpStream;
        use std::time::Duration as StdDuration;
        
        let addr = format!("127.0.0.1:{}", self.socks_port)
            .parse()
            .context("Invalid SOCKS address")?;
        
        TcpStream::connect_timeout(&addr, StdDuration::from_secs(1))
            .context("Failed to connect to SOCKS port")?;
        
        Ok(())
    }

    async fn initialize_proxy_client(&self) -> Result<()> {
        let proxy_url = format!("socks5://127.0.0.1:{}", self.socks_port);
        
        let client = Client::builder()
            .proxy(reqwest::Proxy::all(&proxy_url)
                .context("Failed to create proxy configuration")?)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
            
        *self.client.write().await = Some(client);
        debug!("Initialized HTTP client with SOCKS proxy: {}", proxy_url);
        Ok(())
    }
}

impl Drop for AnyoneService {
    fn drop(&mut self) {
        // Attempt cleanup but don't block
        if let Ok(mut process) = self.process.try_lock() {
            if let Some(mut child) = process.take() {
                let _ = child.start_kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let service = AnyoneService::new(9050, 9051);
        assert_eq!(service.get_socks_port(), 9050);
        assert_eq!(service.get_control_port(), 9051);
        assert!(!service.is_enabled().await);
    }

    #[tokio::test] 
    async fn test_status_transitions() {
        let service = AnyoneService::new(9052, 9053);
        
        assert_eq!(service.get_status().await, ServiceStatus::Stopped);
        
        // Test status updates (without actually starting)
        *service.status.write().await = ServiceStatus::Starting;
        assert_eq!(service.get_status().await, ServiceStatus::Starting);
    }
}
