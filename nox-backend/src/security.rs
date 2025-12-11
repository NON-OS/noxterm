//! NOXTERM Security Module
//!
//! Input sanitization, rate limiting, and security validation.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;
use tracing::warn;

/// Dangerous commands that should be blocked
static BLOCKED_COMMANDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    // Destructive commands
    set.insert("rm -rf /");
    set.insert("rm -rf /*");
    set.insert("rm -fr /");
    set.insert("rm -fr /*");
    set.insert("dd if=/dev/zero of=/dev/sda");
    set.insert("mkfs");
    set.insert("mkfs.ext4 /dev/sda");
    set.insert(":(){ :|:& };:"); // Fork bomb
    set.insert("echo c > /proc/sysrq-trigger");

    // Container escape attempts
    set.insert("nsenter");
    set.insert("docker exec");
    set.insert("docker run --privileged");
    set.insert("mount /dev/sda");

    // Network attacks
    set.insert("nc -e");
    set.insert("ncat -e");
    set.insert("bash -i >& /dev/tcp");
    set.insert("/dev/tcp/");
    set.insert("/dev/udp/");

    set
});

/// Dangerous patterns (regex)
static DANGEROUS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Fork bombs
        Regex::new(r":\(\)\s*\{\s*:\|:&\s*\}\s*;:").unwrap(),
        Regex::new(r"\.0\s*\{\s*\.0\|\.0&\s*\}\s*;\.0").unwrap(),

        // Recursive deletion of root
        Regex::new(r"rm\s+(-[rfR]+\s+)*(/\s*$|/\*|/\s+)").unwrap(),

        // DD to device
        Regex::new(r"dd\s+.*of=/dev/(sd|hd|nvme|vd)[a-z]").unwrap(),

        // Reverse shells
        Regex::new(r"bash\s+-i\s*>&\s*/dev/tcp").unwrap(),
        Regex::new(r"nc\s+.*-e\s+(/bin/)?(ba)?sh").unwrap(),
        Regex::new(r"ncat\s+.*-e\s+(/bin/)?(ba)?sh").unwrap(),
        Regex::new(r"python.*socket.*connect").unwrap(),
        Regex::new(r"perl.*socket.*connect").unwrap(),

        // Container escape attempts
        Regex::new(r"nsenter\s+--target\s+1").unwrap(),
        Regex::new(r"docker\s+.*--privileged").unwrap(),
        Regex::new(r"mount\s+.*proc").unwrap(),
        Regex::new(r"/proc/\d+/(root|ns)").unwrap(),

        // Kernel manipulation
        Regex::new(r"/proc/sys(rq-trigger|/kernel)").unwrap(),
        Regex::new(r"echo\s+.*>\s*/proc/").unwrap(),

        // Cron/persistence attempts
        Regex::new(r"crontab\s+-[er]").unwrap(),
        Regex::new(r"/etc/cron").unwrap(),

        // SSH key injection
        Regex::new(r"\.ssh/authorized_keys").unwrap(),

        // System modification
        Regex::new(r"/etc/(passwd|shadow|sudoers)").unwrap(),
        Regex::new(r"chmod\s+[0-7]*777").unwrap(),
        Regex::new(r"chown\s+root").unwrap(),
    ]
});

/// Path traversal patterns
static PATH_TRAVERSAL_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\.\./").unwrap(),
        Regex::new(r"\.\.\\").unwrap(),
        Regex::new(r"%2e%2e[/\\]").unwrap(),
        Regex::new(r"%252e%252e[/\\]").unwrap(),
        Regex::new(r"\.%00\.").unwrap(),
    ]
});

/// Result of security validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_safe: bool,
    pub reason: Option<String>,
    pub severity: Severity,
    pub blocked_pattern: Option<String>,
}

/// Security event severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Safe,
    Warning,
    Critical,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            is_safe: true,
            reason: None,
            severity: Severity::Safe,
            blocked_pattern: None,
        }
    }
}

/// Validate and sanitize user input
pub fn validate_input(input: &str) -> ValidationResult {
    let input_lower = input.to_lowercase();

    // Check for blocked commands
    for blocked in BLOCKED_COMMANDS.iter() {
        if input_lower.contains(*blocked) {
            warn!("Blocked dangerous command: {}", blocked);
            return ValidationResult {
                is_safe: false,
                reason: Some(format!("Blocked dangerous command pattern detected")),
                severity: Severity::Critical,
                blocked_pattern: Some(blocked.to_string()),
            };
        }
    }

    // Check for dangerous patterns
    for pattern in DANGEROUS_PATTERNS.iter() {
        if pattern.is_match(input) {
            warn!("Blocked dangerous pattern in input");
            return ValidationResult {
                is_safe: false,
                reason: Some("Dangerous command pattern detected".to_string()),
                severity: Severity::Critical,
                blocked_pattern: Some(pattern.to_string()),
            };
        }
    }

    // Check for path traversal
    for pattern in PATH_TRAVERSAL_PATTERNS.iter() {
        if pattern.is_match(input) {
            warn!("Path traversal attempt detected");
            return ValidationResult {
                is_safe: false,
                reason: Some("Path traversal attempt detected".to_string()),
                severity: Severity::Warning,
                blocked_pattern: Some(pattern.to_string()),
            };
        }
    }

    // Check for null bytes (common in path traversal attacks)
    if input.contains('\0') {
        warn!("Null byte injection attempt detected");
        return ValidationResult {
            is_safe: false,
            reason: Some("Null byte injection detected".to_string()),
            severity: Severity::Warning,
            blocked_pattern: Some("\\0".to_string()),
        };
    }

    // Check for excessive length (potential DoS)
    if input.len() > 10000 {
        warn!("Input exceeds maximum length");
        return ValidationResult {
            is_safe: false,
            reason: Some("Input exceeds maximum allowed length".to_string()),
            severity: Severity::Warning,
            blocked_pattern: None,
        };
    }

    ValidationResult::default()
}

/// Sanitize container name
pub fn sanitize_container_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(63) // Docker container name limit
        .collect()
}

/// Validate user ID format
pub fn validate_user_id(user_id: &str) -> bool {
    // User ID should be alphanumeric with underscores/hyphens, max 255 chars
    if user_id.is_empty() || user_id.len() > 255 {
        return false;
    }

    user_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Validate container image name
pub fn validate_image_name(image: &str) -> bool {
    // Basic validation for Docker image names
    if image.is_empty() || image.len() > 255 {
        return false;
    }

    // Must not contain dangerous characters
    let invalid_chars = ['$', '`', '|', ';', '&', '>', '<', '\\', '"', '\''];
    !image.chars().any(|c| invalid_chars.contains(&c))
}

/// Extract client IP from request headers (supports proxies)
pub fn extract_client_ip(
    forwarded_for: Option<&str>,
    real_ip: Option<&str>,
    remote_addr: Option<&str>,
) -> Option<String> {
    // Try X-Forwarded-For first (first IP in chain)
    if let Some(xff) = forwarded_for {
        if let Some(first_ip) = xff.split(',').next() {
            let ip = first_ip.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    // Try X-Real-IP
    if let Some(real) = real_ip {
        if !real.is_empty() {
            return Some(real.to_string());
        }
    }

    // Fall back to remote address
    remote_addr.map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safe_input() {
        let result = validate_input("ls -la");
        assert!(result.is_safe);
    }

    #[test]
    fn test_block_rm_rf() {
        let result = validate_input("rm -rf /");
        assert!(!result.is_safe);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_block_fork_bomb() {
        let result = validate_input(":(){ :|:& };:");
        assert!(!result.is_safe);
    }

    #[test]
    fn test_block_path_traversal() {
        let result = validate_input("cat ../../../etc/passwd");
        assert!(!result.is_safe);
    }

    #[test]
    fn test_validate_user_id() {
        assert!(validate_user_id("user123"));
        assert!(validate_user_id("user_name"));
        assert!(validate_user_id("user-name"));
        assert!(!validate_user_id(""));
        assert!(!validate_user_id("user;id"));
    }

    #[test]
    fn test_validate_image_name() {
        assert!(validate_image_name("ubuntu:22.04"));
        assert!(validate_image_name("nginx:latest"));
        assert!(!validate_image_name("ubuntu; rm -rf /"));
        assert!(!validate_image_name(""));
    }

    #[test]
    fn test_sanitize_container_name() {
        assert_eq!(sanitize_container_name("my-container_1"), "my-container_1");
        assert_eq!(sanitize_container_name("bad;name"), "badname");
    }

    #[test]
    fn test_extract_client_ip() {
        assert_eq!(
            extract_client_ip(Some("1.2.3.4, 5.6.7.8"), None, None),
            Some("1.2.3.4".to_string())
        );
        assert_eq!(
            extract_client_ip(None, Some("1.2.3.4"), None),
            Some("1.2.3.4".to_string())
        );
        assert_eq!(
            extract_client_ip(None, None, Some("1.2.3.4:12345")),
            Some("1.2.3.4:12345".to_string())
        );
    }
}
