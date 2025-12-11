use std::process::{Command, Stdio};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{sleep, Duration, timeout};
use tracing::{info, warn, debug};
use reqwest::Client;
use tokio::process::{Child, Command as TokioCommand};
use anyhow::{Result, Context};

/// NOX Rust, Anyone Protocol service manager
/// Cross-platform support for macOS, Linux, and Windows
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

        // Ensure Node.js is installed (auto-install if needed)
        self.ensure_nodejs_installed().await?;

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
                Ok(Ok(exit_status)) => {
                    debug!("Anyone process exited with status: {}", exit_status);
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

    /// Public method to ensure all prerequisites are installed
    /// Called at startup to pre-install Node.js and Anyone SDK
    pub async fn ensure_prerequisites(&self) -> Result<()> {
        // Ensure Node.js is installed (auto-install if needed)
        self.ensure_nodejs_installed().await?;

        // Install Anyone client if needed
        self.ensure_anyone_client_installed().await?;

        Ok(())
    }

    // ========================================================================
    // Cross-platform Node.js auto-installation
    // ========================================================================

    /// Check if Node.js is installed, if not, attempt to install it
    async fn ensure_nodejs_installed(&self) -> Result<()> {
        let (node_cmd, npm_cmd) = Self::get_node_commands();

        // Check if Node.js is already installed
        if let Ok(output) = Command::new(&node_cmd).args(["--version"]).output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Node.js version: {}", version.trim());

                // Also verify npm
                if let Ok(npm_output) = Command::new(&npm_cmd).args(["--version"]).output() {
                    if npm_output.status.success() {
                        let npm_version = String::from_utf8_lossy(&npm_output.stdout);
                        info!("npm version: {}", npm_version.trim());
                        return Ok(());
                    }
                }
            }
        }

        // Node.js not found - attempt to install
        info!("Node.js not found. Attempting to install...");

        if cfg!(target_os = "macos") {
            self.install_nodejs_macos().await?;
        } else if cfg!(target_os = "linux") {
            self.install_nodejs_linux().await?;
        } else if cfg!(target_os = "windows") {
            self.install_nodejs_windows().await?;
        } else {
            return Err(anyhow::anyhow!(
                "Unsupported platform. Please install Node.js manually from https://nodejs.org/"
            ));
        }

        // Verify installation
        self.verify_nodejs_installation().await
    }

    /// Get platform-specific node/npm command names
    fn get_node_commands() -> (String, String) {
        if cfg!(target_os = "windows") {
            ("node.exe".to_string(), "npm.cmd".to_string())
        } else {
            ("node".to_string(), "npm".to_string())
        }
    }

    /// Install Node.js on macOS using Homebrew
    async fn install_nodejs_macos(&self) -> Result<()> {
        // Check if Homebrew is installed
        let brew_installed = Command::new("which")
            .arg("brew")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if brew_installed {
            info!("Installing Node.js via Homebrew...");
            let status = Command::new("brew")
                .args(["install", "node"])
                .status();

            if status.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via Homebrew");
                return Ok(());
            }
        }

        // Try using the official installer via curl
        info!("Installing Node.js via official script...");

        // Check for nvm
        let nvm_installed = Command::new("bash")
            .args(["-c", "command -v nvm"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !nvm_installed {
            // Install nvm first
            info!("Installing nvm (Node Version Manager)...");
            let nvm_install = Command::new("bash")
                .args(["-c", "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash"])
                .status();

            if nvm_install.map(|s| s.success()).unwrap_or(false) {
                info!("nvm installed. Installing Node.js...");

                // Install latest LTS Node.js using nvm
                let node_install = Command::new("bash")
                    .args(["-c", "source ~/.nvm/nvm.sh && nvm install --lts && nvm use --lts"])
                    .status();

                if node_install.map(|s| s.success()).unwrap_or(false) {
                    info!("✅ Node.js installed via nvm");
                    return Ok(());
                }
            }
        }

        // Fallback: download and run official pkg installer
        warn!("Could not auto-install Node.js on macOS.");
        Err(anyhow::anyhow!(
            "Node.js auto-installation failed on macOS.\n\
            Please install manually using one of these methods:\n\
            1. Homebrew: brew install node\n\
            2. Official installer: https://nodejs.org/\n\
            3. nvm: curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash"
        ))
    }

    /// Install Node.js on Linux using package managers
    async fn install_nodejs_linux(&self) -> Result<()> {
        // Try NodeSource setup script (works on most distros)
        info!("Installing Node.js via NodeSource...");

        let nodesource_result = Command::new("bash")
            .args(["-c", "curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -"])
            .status();

        if nodesource_result.map(|s| s.success()).unwrap_or(false) {
            // Now install via apt
            if Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false) {
                let apt_result = Command::new("sudo")
                    .args(["apt-get", "install", "-y", "nodejs"])
                    .status();

                if apt_result.map(|s| s.success()).unwrap_or(false) {
                    info!("✅ Node.js installed via apt");
                    return Ok(());
                }
            }
        }

        // Try apt directly (Ubuntu/Debian)
        if Command::new("which").arg("apt-get").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via apt-get...");
            let _ = Command::new("sudo").args(["apt-get", "update"]).status();
            let apt_result = Command::new("sudo")
                .args(["apt-get", "install", "-y", "nodejs", "npm"])
                .status();

            if apt_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via apt");
                return Ok(());
            }
        }

        // Try dnf (Fedora/RHEL)
        if Command::new("which").arg("dnf").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via dnf...");
            let dnf_result = Command::new("sudo")
                .args(["dnf", "install", "-y", "nodejs", "npm"])
                .status();

            if dnf_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via dnf");
                return Ok(());
            }
        }

        // Try yum (CentOS/older RHEL)
        if Command::new("which").arg("yum").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via yum...");
            let yum_result = Command::new("sudo")
                .args(["yum", "install", "-y", "nodejs", "npm"])
                .status();

            if yum_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via yum");
                return Ok(());
            }
        }

        // Try pacman (Arch)
        if Command::new("which").arg("pacman").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via pacman...");
            let pacman_result = Command::new("sudo")
                .args(["pacman", "-S", "--noconfirm", "nodejs", "npm"])
                .status();

            if pacman_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via pacman");
                return Ok(());
            }
        }

        // Try zypper (openSUSE)
        if Command::new("which").arg("zypper").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via zypper...");
            let zypper_result = Command::new("sudo")
                .args(["zypper", "install", "-y", "nodejs", "npm"])
                .status();

            if zypper_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via zypper");
                return Ok(());
            }
        }

        // Fallback: use nvm
        info!("Trying nvm installation...");
        let nvm_install = Command::new("bash")
            .args(["-c", "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash && source ~/.nvm/nvm.sh && nvm install --lts"])
            .status();

        if nvm_install.map(|s| s.success()).unwrap_or(false) {
            info!("✅ Node.js installed via nvm");
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Node.js auto-installation failed on Linux.\n\
            Please install manually using your package manager:\n\
            - Debian/Ubuntu: sudo apt install nodejs npm\n\
            - Fedora: sudo dnf install nodejs npm\n\
            - Arch: sudo pacman -S nodejs npm\n\
            - Or via nvm: curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash"
        ))
    }

    /// Install Node.js on Windows
    async fn install_nodejs_windows(&self) -> Result<()> {
        // Check common installation paths
        let common_paths = [
            r"C:\Program Files\nodejs\node.exe",
            r"C:\Program Files (x86)\nodejs\node.exe",
        ];

        for path in &common_paths {
            if Path::new(path).exists() {
                info!("Found Node.js at: {}", path);
                // It exists but might not be in PATH - try to add it
                warn!("Node.js found but not in PATH. Please add to PATH or restart your terminal.");
                return Ok(());
            }
        }

        // Check if winget is available (Windows Package Manager)
        if Command::new("winget").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via winget...");
            let winget_result = Command::new("winget")
                .args(["install", "--id", "OpenJS.NodeJS.LTS", "-e", "--silent"])
                .status();

            if winget_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via winget");
                info!("Please restart your terminal to use Node.js");
                return Ok(());
            }
        }

        // Check if Chocolatey is available
        if Command::new("choco").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via Chocolatey...");
            let choco_result = Command::new("choco")
                .args(["install", "nodejs-lts", "-y"])
                .status();

            if choco_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via Chocolatey");
                info!("Please restart your terminal to use Node.js");
                return Ok(());
            }
        }

        // Check if Scoop is available
        if Command::new("scoop").arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            info!("Installing Node.js via Scoop...");
            let scoop_result = Command::new("scoop")
                .args(["install", "nodejs-lts"])
                .status();

            if scoop_result.map(|s| s.success()).unwrap_or(false) {
                info!("✅ Node.js installed via Scoop");
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Node.js auto-installation failed on Windows.\n\
            Please install manually using one of these methods:\n\
            1. Official installer: https://nodejs.org/\n\
            2. winget: winget install OpenJS.NodeJS.LTS\n\
            3. Chocolatey: choco install nodejs-lts\n\
            4. Scoop: scoop install nodejs-lts"
        ))
    }

    /// Verify Node.js installation after attempting to install
    async fn verify_nodejs_installation(&self) -> Result<()> {
        let (node_cmd, npm_cmd) = Self::get_node_commands();

        // Give the system a moment to update PATH
        sleep(Duration::from_secs(1)).await;

        let node_output = Command::new(&node_cmd)
            .args(["--version"])
            .output();

        match node_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("✅ Node.js verified: {}", version.trim());
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Node.js installation could not be verified.\n\
                    Please restart your terminal and try again, or install manually from https://nodejs.org/"
                ));
            }
        }

        let npm_output = Command::new(&npm_cmd)
            .args(["--version"])
            .output();

        match npm_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("✅ npm verified: {}", version.trim());
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "npm installation could not be verified.\n\
                    Please restart your terminal and try again."
                ));
            }
        }

        Ok(())
    }

    // ========================================================================
    // Anyone Protocol client installation
    // ========================================================================

    async fn ensure_anyone_client_installed(&self) -> Result<()> {
        let (_, npm_cmd) = Self::get_node_commands();
        let npx_cmd = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };

        // First try to check if it's globally installed
        let check_output = Command::new(&npm_cmd)
            .args(["list", "-g", "@anyone-protocol/anyone-client"])
            .output();

        if let Ok(output) = check_output {
            if output.status.success() {
                info!("Anyone Protocol client already installed globally");
                return Ok(());
            }
        }

        // Also check if npx can find it (might be cached or local)
        let npx_check = Command::new(npx_cmd)
            .args(["--yes", "@anyone-protocol/anyone-client", "--version"])
            .output();

        if let Ok(output) = npx_check {
            if output.status.success() {
                info!("Anyone Protocol client available via npx");
                return Ok(());
            }
        }

        info!("Installing Anyone Protocol client globally...");
        info!("Running: npm install -g @anyone-protocol/anyone-client");

        let install_output = Command::new(&npm_cmd)
            .args(["install", "-g", "@anyone-protocol/anyone-client"])
            .output()
            .context("Failed to execute npm install. Check npm permissions.")?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            let stdout = String::from_utf8_lossy(&install_output.stdout);

            // Provide helpful error message based on common issues
            let help_msg = if stderr.contains("EACCES") || stderr.contains("permission denied") {
                if cfg!(target_os = "windows") {
                    "\n\nPermission denied. Try running as Administrator or use:\n  npm config set prefix %USERPROFILE%\\.npm-global"
                } else {
                    "\n\nPermission denied. Fix with one of these:\n  \
                    1. Use nvm (recommended): https://github.com/nvm-sh/nvm\n  \
                    2. Fix npm permissions: mkdir ~/.npm-global && npm config set prefix '~/.npm-global'\n  \
                    3. Or run with sudo (not recommended): sudo npm install -g @anyone-protocol/anyone-client"
                }
            } else {
                ""
            };

            return Err(anyhow::anyhow!(
                "Failed to install Anyone Protocol client.\nstdout: {}\nstderr: {}{}",
                stdout.trim(),
                stderr.trim(),
                help_msg
            ));
        }

        info!("✅ Successfully installed Anyone Protocol client");
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
        let npx_cmd = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };

        info!("Starting Anyone Protocol client...");
        info!("  SOCKS proxy: 127.0.0.1:{}", self.socks_port);
        info!("  Control port: 127.0.0.1:{}", self.control_port);

        let child = TokioCommand::new(npx_cmd)
            .args([
                "--yes",  // Auto-install if needed
                "@anyone-protocol/anyone-client",
                "-s", &self.socks_port.to_string(),
                "-c", &self.control_port.to_string(),
                "-v"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context(format!(
                "Failed to spawn Anyone client process.\n\
                Make sure Node.js and npm are installed and in your PATH."
            ))?;

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

    #[tokio::test]
    async fn test_get_node_commands() {
        let (node_cmd, npm_cmd) = AnyoneService::get_node_commands();

        #[cfg(target_os = "windows")]
        {
            assert_eq!(node_cmd, "node.exe");
            assert_eq!(npm_cmd, "npm.cmd");
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(node_cmd, "node");
            assert_eq!(npm_cmd, "npm");
        }
    }
}
