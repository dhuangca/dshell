/// Permission management for secure shell

use std::collections::HashSet;
use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allowed,
    Denied,
    AskEveryTime,
}

pub struct PermissionManager {
    // Global permission for env access
    env_access_permission: Permission,
    // Specific environment variables that are allowed
    allowed_env_vars: HashSet<String>,
    // Specific environment variables that are denied
    denied_env_vars: HashSet<String>,
    // Filesystem paths that are allowed
    allowed_paths: HashSet<String>,
    // Filesystem paths that are denied
    denied_paths: HashSet<String>,
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut allowed_env_vars = HashSet::new();

        // Allow common safe environment variables by default
        // These are needed for basic shell and tool functionality
        allowed_env_vars.insert("HOME".to_string());
        allowed_env_vars.insert("PATH".to_string());
        allowed_env_vars.insert("USER".to_string());
        allowed_env_vars.insert("SHELL".to_string());
        allowed_env_vars.insert("TERM".to_string());
        allowed_env_vars.insert("LANG".to_string());
        allowed_env_vars.insert("EDITOR".to_string());
        allowed_env_vars.insert("COLORTERM".to_string());

        // Rust toolchain environment variables
        allowed_env_vars.insert("RUSTUP_HOME".to_string());
        allowed_env_vars.insert("CARGO_HOME".to_string());
        allowed_env_vars.insert("RUST_BACKTRACE".to_string());
        allowed_env_vars.insert("RUSTC".to_string());
        allowed_env_vars.insert("RUSTDOC".to_string());

        PermissionManager {
            env_access_permission: Permission::AskEveryTime,
            allowed_env_vars,
            denied_env_vars: HashSet::new(),
            allowed_paths: HashSet::new(),
            denied_paths: HashSet::new(),
        }
    }

    /// Set global environment access permission
    pub fn set_env_access(&mut self, permission: Permission) {
        self.env_access_permission = permission;
    }

    /// Check if a specific environment variable is allowed
    pub fn check_env_var(&self, var_name: &str) -> Permission {
        // Check if specifically denied
        if self.denied_env_vars.contains(var_name) {
            return Permission::Denied;
        }

        // Check if specifically allowed
        if self.allowed_env_vars.contains(var_name) {
            return Permission::Allowed;
        }

        // Fall back to global permission
        self.env_access_permission.clone()
    }

    /// Allow a specific environment variable
    pub fn allow_env_var(&mut self, var_name: String) {
        self.denied_env_vars.remove(&var_name);
        self.allowed_env_vars.insert(var_name);
    }

    /// Deny a specific environment variable
    pub fn deny_env_var(&mut self, var_name: String) {
        self.allowed_env_vars.remove(&var_name);
        self.denied_env_vars.insert(var_name);
    }

    /// Get filtered environment variables based on permissions
    pub fn get_allowed_env_vars(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();

        for (key, value) in env::vars() {
            match self.check_env_var(&key) {
                Permission::Allowed => {
                    result.push((key, value));
                }
                Permission::Denied => {
                    // Skip denied variables
                }
                Permission::AskEveryTime => {
                    // For now, redact variables that need permission
                    result.push((key, "[REDACTED - use 'allow' command]".to_string()));
                }
            }
        }

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Get list of allowed environment variable names
    pub fn list_allowed_env_vars(&self) -> Vec<String> {
        let mut vars: Vec<String> = self.allowed_env_vars.iter().cloned().collect();
        vars.sort();
        vars
    }

    /// Get list of denied environment variable names
    pub fn list_denied_env_vars(&self) -> Vec<String> {
        let mut vars: Vec<String> = self.denied_env_vars.iter().cloned().collect();
        vars.sort();
        vars
    }

    /// Allow a specific filesystem path
    pub fn allow_path(&mut self, path: String) {
        // Expand tilde if present
        let expanded_path = Self::expand_tilde(&path);
        self.denied_paths.remove(&expanded_path);
        self.allowed_paths.insert(expanded_path);
    }

    /// Deny a specific filesystem path
    pub fn deny_path(&mut self, path: String) {
        // Expand tilde if present
        let expanded_path = Self::expand_tilde(&path);
        self.allowed_paths.remove(&expanded_path);
        self.denied_paths.insert(expanded_path);
    }

    /// Get list of allowed paths
    pub fn list_allowed_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.allowed_paths.iter().cloned().collect();
        paths.sort();
        paths
    }

    /// Get allowed paths as a reference for Landlock
    pub fn get_allowed_paths(&self) -> &HashSet<String> {
        &self.allowed_paths
    }

    /// Get list of denied paths
    pub fn list_denied_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.denied_paths.iter().cloned().collect();
        paths.sort();
        paths
    }

    /// Get security status report
    pub fn get_status(&self) -> Vec<String> {
        let mut output = vec![
            "Security Status:".to_string(),
            "".to_string(),
            format!("Global Env Access: {:?}", self.env_access_permission),
            format!("Allowed Env Vars: {}", self.allowed_env_vars.len()),
            format!("Denied Env Vars: {}", self.denied_env_vars.len()),
        ];

        if !self.allowed_env_vars.is_empty() {
            output.push("".to_string());
            output.push("Explicitly Allowed Env Vars:".to_string());
            for var in self.list_allowed_env_vars() {
                output.push(format!("  ✓ {}", var));
            }
        }

        if !self.denied_env_vars.is_empty() {
            output.push("".to_string());
            output.push("Explicitly Denied Env Vars:".to_string());
            for var in self.list_denied_env_vars() {
                output.push(format!("  ✗ {}", var));
            }
        }

        if !self.allowed_paths.is_empty() {
            output.push("".to_string());
            output.push("Allowed Filesystem Paths:".to_string());
            for path in self.list_allowed_paths() {
                output.push(format!("  ✓ {}", path));
            }
        }

        if !self.denied_paths.is_empty() {
            output.push("".to_string());
            output.push("Denied Filesystem Paths:".to_string());
            for path in self.list_denied_paths() {
                output.push(format!("  ✗ {}", path));
            }
        }

        output
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

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
