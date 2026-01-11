/// Landlock-based filesystem isolation
///
/// This module provides filesystem access restriction using Linux Landlock LSM.
/// Requires Linux kernel 5.13 or later.

use landlock::*;
use std::env;
use std::io;
use std::path::{Path, PathBuf};

/// Isolation status indicating what level of protection is active
#[derive(Debug, Clone, PartialEq)]
pub enum IsolationStatus {
    /// Filesystem access is fully restricted by kernel
    FullyEnforced,
    /// Filesystem access is partially restricted (some features not available)
    PartiallyEnforced,
    /// Filesystem restrictions could not be applied
    NotEnforced,
    /// Landlock is not available on this system
    NotAvailable,
}

impl IsolationStatus {
    #[allow(dead_code)]
    pub fn is_enforced(&self) -> bool {
        matches!(self, IsolationStatus::FullyEnforced | IsolationStatus::PartiallyEnforced)
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &str {
        match self {
            IsolationStatus::FullyEnforced => "Filesystem fully restricted",
            IsolationStatus::PartiallyEnforced => "Filesystem partially restricted",
            IsolationStatus::NotEnforced => "Filesystem restrictions not enforced",
            IsolationStatus::NotAvailable => "Landlock not available on this system",
        }
    }
}

/// Landlock filesystem isolation manager
pub struct LandlockIsolation {
    work_dir: PathBuf,
}

impl LandlockIsolation {
    /// Create a new Landlock isolation manager for the given working directory
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Create isolation manager for current directory
    #[allow(dead_code)]
    pub fn for_current_dir() -> io::Result<Self> {
        let work_dir = env::current_dir()?;
        Ok(Self::new(work_dir))
    }

    /// Check if Landlock is available on this system
    pub fn is_available() -> bool {
        // Check if we can get any supported ABI version
        Self::get_abi_version().is_some()
    }

    /// Get the highest supported Landlock ABI version
    pub fn get_abi_version() -> Option<ABI> {
        // Try versions from newest to oldest
        // We test by trying to create and instantiate a ruleset
        for abi in &[ABI::V4, ABI::V3, ABI::V2, ABI::V1] {
            // Try to create a simple ruleset to test if this ABI is supported
            let result = Ruleset::default()
                .handle_access(AccessFs::from_all(*abi))
                .and_then(|r| r.create());

            if result.is_ok() {
                return Some(*abi);
            }
        }
        None
    }

    /// Apply filesystem restrictions to current process
    ///
    /// This will restrict the process (and all its children) to only access
    /// files within the working directory and any additional allowed paths.
    /// Cannot be undone once applied.
    ///
    /// Returns the isolation status indicating if restrictions were applied.
    pub fn restrict_filesystem(&self, additional_allowed_paths: &[String]) -> io::Result<IsolationStatus> {
        // Check if Landlock is available
        let abi = match Self::get_abi_version() {
            Some(abi) => abi,
            None => return Ok(IsolationStatus::NotAvailable),
        };

        // Define the access rights we want to allow within the working directory
        let fs_access = AccessFs::from_all(abi);

        // Create a ruleset that handles filesystem access
        let mut ruleset = Ruleset::default()
            .handle_access(fs_access)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to create ruleset: {}", e),
                )
            })?
            .create()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Failed to create ruleset (permission denied): {}", e),
                )
            })?;

        // Add rules allowing access to necessary directories
        use std::fs::File;
        use std::path::Path;

        // Define read-only access for system paths (execute and read, no write)
        use AccessFs as A;
        let ro_access = A::Execute | A::ReadFile | A::ReadDir;

        // System paths (read-only + execute)
        let readonly_paths = vec![
            Path::new("/usr"),                 // System binaries and libraries
            Path::new("/bin"),                 // Essential binaries
            Path::new("/lib"),                 // Essential libraries
            Path::new("/lib64"),               // 64-bit libraries
            Path::new("/etc"),                 // System configuration (needed for DNS, hosts, etc.)
            Path::new("/dev"),                 // Device files
            Path::new("/proc"),                // Process information
            Path::new("/sys"),                 // System information
            Path::new("/run"),                 // Runtime data (needed for systemd DNS resolution)
        ];

        // Paths that need write access for temporary files
        let readwrite_system_paths = vec![
            Path::new("/tmp"),                 // Temporary files
            Path::new("/var/tmp"),             // Temporary files
        ];

        // Add read-only system paths
        for path in readonly_paths {
            if path.exists() {
                if let Ok(dir_fd) = File::open(path) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(dir_fd, ro_access))
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to add read-only rule for {}: {}", path.display(), e),
                            )
                        })?;
                }
            }
        }

        // Add read-write system paths (for temp files)
        for path in readwrite_system_paths {
            if path.exists() {
                if let Ok(dir_fd) = File::open(path) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(dir_fd, fs_access))
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to add read-write rule for {}: {}", path.display(), e),
                            )
                        })?;
                }
            }
        }

        // Add allowed paths (from config and allowpath command)
        for path_str in additional_allowed_paths {
            let expanded_path = Self::expand_tilde(path_str);
            let path = Path::new(&expanded_path);

            if path.exists() {
                if let Ok(fd) = File::open(path) {
                    ruleset = ruleset
                        .add_rule(PathBeneath::new(fd, fs_access))
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                format!("Failed to add allowed path rule for {}: {}", path.display(), e),
                            )
                        })?;
                }
            }
        }

        // Add full access to working directory (read, write, execute)
        let dir_fd = File::open(&self.work_dir).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open working directory: {}", e),
            )
        })?;

        ruleset = ruleset
            .add_rule(PathBeneath::new(dir_fd, fs_access))
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to add path rule: {}", e),
                )
            })?;

        // Apply the restrictions to the current thread
        // After this call, this process can only access files in work_dir
        let status = ruleset.restrict_self().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to restrict process: {}", e),
            )
        })?;

        // Convert Landlock's status to our IsolationStatus
        let isolation_status = match status.ruleset {
            RulesetStatus::FullyEnforced => IsolationStatus::FullyEnforced,
            RulesetStatus::PartiallyEnforced => IsolationStatus::PartiallyEnforced,
            RulesetStatus::NotEnforced => IsolationStatus::NotEnforced,
        };

        Ok(isolation_status)
    }

    /// Get the working directory that will be accessible after restriction
    #[allow(dead_code)]
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Expand tilde (~) in path to home directory
    fn expand_tilde(path: &str) -> String {
        if path.starts_with("~/") || path == "~" {
            if let Ok(home) = env::var("HOME") {
                return path.replacen("~", &home, 1);
            }
        }
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_landlock_availability() {
        // Just check that the function doesn't panic
        let available = LandlockIsolation::is_available();
        println!("Landlock available: {}", available);
    }

    #[test]
    fn test_get_abi_version() {
        let version = LandlockIsolation::get_abi_version();
        println!("Landlock ABI version: {:?}", version);
    }

    #[test]
    fn test_create_isolation() {
        let temp_dir = std::env::temp_dir();
        let isolation = LandlockIsolation::new(temp_dir.clone());
        assert_eq!(isolation.work_dir(), temp_dir.as_path());
    }

    #[test]
    #[ignore] // Only run manually as it actually restricts the process
    fn test_filesystem_restriction() {
        // Create a test directory
        let test_dir = std::env::temp_dir().join("dshell_landlock_test");
        fs::create_dir_all(&test_dir).unwrap();

        // Create test files
        let allowed_file = test_dir.join("allowed.txt");
        let restricted_file = std::env::temp_dir().join("restricted.txt");

        fs::write(&allowed_file, "allowed content").unwrap();
        fs::write(&restricted_file, "restricted content").unwrap();

        // Apply restrictions
        let isolation = LandlockIsolation::new(test_dir.clone());
        let status = isolation.restrict_filesystem(&[]).unwrap();

        println!("Isolation status: {:?}", status);

        if status.is_enforced() {
            // Should be able to read allowed file
            let content = fs::read_to_string(&allowed_file);
            assert!(content.is_ok());

            // Should NOT be able to read restricted file
            let content = fs::read_to_string(&restricted_file);
            assert!(content.is_err());

            println!("✓ Filesystem isolation is working!");
        } else {
            println!("⚠ Filesystem isolation not enforced on this system");
        }

        // Cleanup
        let _ = fs::remove_file(&allowed_file);
        let _ = fs::remove_dir(&test_dir);
    }
}
