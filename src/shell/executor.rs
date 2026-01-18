/// Command execution module

use super::parser::ParsedCommand;
use crate::config::Config;
use crate::security::{PermissionManager, LandlockIsolation, IsolationStatus};
use std::collections::HashMap;
use std::env;
use std::io;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    Captured,    // Capture output for display
    Interactive, // Run with full terminal access
}

#[derive(Debug)]
pub struct CommandResult {
    pub output: Vec<String>,
}

pub struct Executor;

impl Executor {
    /// Determine if a command should run in interactive mode
    pub fn execution_mode(cmd: &ParsedCommand, config: &Config) -> ExecutionMode {
        if config.interactive_commands.iter().any(|c| c == &cmd.command) {
            ExecutionMode::Interactive
        } else {
            ExecutionMode::Captured
        }
    }

    /// Execute a command and capture its output with filtered environment
    pub fn execute_captured(cmd: &ParsedCommand, permissions: &PermissionManager, custom_env: &HashMap<String, String>) -> CommandResult {
        let mut command = Command::new(&cmd.command);
        command.args(&cmd.args);

        // Clear all environment variables and only add allowed ones
        command.env_clear();

        // Add custom env vars first
        for (key, value) in custom_env {
            command.env(key, value);
        }

        // Add system env vars (permissions respected)
        for (key, value) in permissions.get_allowed_env_vars() {
            // Only add if not redacted and not already in custom_env
            if !value.starts_with("[REDACTED") && !custom_env.contains_key(&key) {
                command.env(key, value);
            }
        }

        match command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) => {
                let mut lines = Vec::new();

                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    lines.push(line.to_string());
                }

                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    // Don't prefix with "Error:" - stderr contains all diagnostic output,
                    // not just errors (progress messages, warnings, info, etc.)
                    lines.push(line.to_string());
                }

                CommandResult {
                    output: lines,
                }
            }
            Err(e) => {
                let mut error_msg = vec![format!("dshell: {}: {}", cmd.command, e)];

                // Add helpful hints for common errors
                if e.kind() == io::ErrorKind::NotFound {
                    error_msg.push(format!("  Hint: Command '{}' not found in PATH", cmd.command));
                    error_msg.push(format!("  Current PATH: {}", std::env::var("PATH").unwrap_or_else(|_| "not set".to_string())));
                } else if e.kind() == io::ErrorKind::PermissionDenied {
                    error_msg.push(format!("  Hint: Permission denied for '{}'", cmd.command));
                    error_msg.push("  Check if the file is executable".to_string());
                }

                CommandResult {
                    output: error_msg,
                }
            },
        }
    }

    /// Execute a command in interactive mode with Landlock filesystem isolation
    ///
    /// This function will:
    /// 1. Fork a child process
    /// 2. Apply Landlock restrictions in the child (restricts to current directory)
    /// 3. Execute the command with filtered environment
    /// 4. Return the exit code
    ///
    /// If Landlock is not available, falls back to warning-only mode.
    pub fn execute_interactive(cmd: &ParsedCommand, permissions: &PermissionManager, custom_env: &HashMap<String, String>, config: &Config) -> io::Result<IsolationStatus> {
        let work_dir = env::current_dir()?;

        // Fork the process
        let pid = unsafe { libc::fork() };

        match pid {
            -1 => {
                // Fork failed
                let err = io::Error::last_os_error();
                eprintln!("Error: Failed to fork process: {}", err);
                Err(err)
            }
            0 => {
                // Child process - apply Landlock restrictions and execute command
                let _isolation_status = Self::execute_in_isolated_child(cmd, permissions, custom_env, work_dir, config);

                // If we reach here, exec failed
                eprintln!("Error: exec failed for command: {}", cmd.command);
                std::process::exit(127);
            }
            child_pid => {
                // Parent process - wait for child to complete
                let mut status: libc::c_int = 0;
                let wait_result = unsafe { libc::waitpid(child_pid, &mut status, 0) };

                if wait_result == -1 {
                    let err = io::Error::last_os_error();
                    eprintln!("Error: waitpid failed: {}", err);
                    return Err(err);
                }

                // Check if child exited with an error
                if libc::WIFEXITED(status) {
                    let exit_code = libc::WEXITSTATUS(status);
                    if exit_code == 127 {
                        eprintln!("Error: Command not found or failed to execute: {}", cmd.command);
                    }
                }

                // For now, return NotEnforced as parent doesn't know child's isolation status
                // We could use a pipe to communicate this back if needed
                Ok(IsolationStatus::NotEnforced)
            }
        }
    }

    /// Execute command in isolated child process (called after fork)
    fn execute_in_isolated_child(
        cmd: &ParsedCommand,
        permissions: &PermissionManager,
        custom_env: &HashMap<String, String>,
        work_dir: std::path::PathBuf,
        config: &Config,
    ) -> IsolationStatus {
        // Apply Landlock restrictions with allowed paths from permissions and config
        let isolation = LandlockIsolation::new(work_dir.clone());

        // Combine allowed paths from permissions and config
        let mut allowed_paths: Vec<String> = permissions.get_allowed_paths().iter().cloned().collect();

        // Add additional paths from config, but exclude any denied paths
        let denied_paths_set: std::collections::HashSet<String> =
            permissions.list_denied_paths().into_iter().collect();

        for path in &config.additional_allowed_paths {
            // Expand tilde for comparison
            let expanded = if path.starts_with("~/") || path == "~" {
                if let Ok(home) = std::env::var("HOME") {
                    path.replacen("~", &home, 1)
                } else {
                    path.clone()
                }
            } else {
                path.clone()
            };

            // Only add if not denied
            if !denied_paths_set.contains(&expanded) {
                allowed_paths.push(path.clone());
            }
        }

        let isolation_status = match isolation.restrict_filesystem(&allowed_paths) {
            Ok(status) => {
                // Print status message
                match &status {
                    IsolationStatus::FullyEnforced => {
                        eprintln!("🔒 Filesystem isolated to: {}", work_dir.display());
                        if !allowed_paths.is_empty() {
                            eprintln!("   Plus {} additional allowed path(s)", allowed_paths.len());
                        }
                    }
                    IsolationStatus::PartiallyEnforced => {
                        eprintln!("⚠️  Filesystem partially isolated to: {}", work_dir.display());
                        if !allowed_paths.is_empty() {
                            eprintln!("   Plus {} additional allowed path(s)", allowed_paths.len());
                        }
                    }
                    IsolationStatus::NotEnforced => {
                        eprintln!("⚠️  Warning: Could not enforce filesystem isolation");
                    }
                    IsolationStatus::NotAvailable => {
                        eprintln!("⚠️  Warning: Landlock not available (requires Linux 5.13+)");
                        eprintln!("    Process can access files outside: {}", work_dir.display());
                    }
                }
                status
            }
            Err(e) => {
                eprintln!("⚠️  Warning: Failed to apply filesystem isolation: {}", e);
                eprintln!("    Process can access files outside: {}", work_dir.display());
                IsolationStatus::NotEnforced
            }
        };

        // Build command with filtered environment
        let mut command = Command::new(&cmd.command);
        command.args(&cmd.args);
        command.env_clear();

        // Set PWD and working directory
        if let Some(pwd) = work_dir.to_str() {
            command.env("PWD", pwd);
            command.env("DSHELL_RESTRICTED", "1");
            command.env("DSHELL_RESTRICTED_ROOT", pwd);

            // Add isolation status to environment
            command.env("DSHELL_ISOLATION_STATUS", match isolation_status {
                IsolationStatus::FullyEnforced => "fully_enforced",
                IsolationStatus::PartiallyEnforced => "partially_enforced",
                IsolationStatus::NotEnforced => "not_enforced",
                IsolationStatus::NotAvailable => "not_available",
            });
        }
        command.current_dir(&work_dir);

        // Add custom env vars
        for (key, value) in custom_env {
            command.env(key, value);
        }

        // Add system env vars (permissions respected)
        for (key, value) in permissions.get_allowed_env_vars() {
            if !value.starts_with("[REDACTED") && !custom_env.contains_key(&key) {
                command.env(key, value);
            }
        }

        // Execute the command
        let status = match command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
        {
            Ok(s) => s.code().unwrap_or(-1),
            Err(e) => {
                eprintln!("\ndshell: Failed to execute '{}': {}", cmd.command, e);

                // Add helpful hints for common errors
                match e.kind() {
                    io::ErrorKind::NotFound => {
                        eprintln!("  Hint: Command '{}' not found", cmd.command);
                        eprintln!("  Possible reasons:");
                        eprintln!("    - Command is not installed");
                        eprintln!("    - Command is not in PATH");
                        eprintln!("    - Landlock isolation preventing access to the binary");
                        if let Ok(path) = std::env::var("PATH") {
                            eprintln!("  Current PATH: {}", path);
                        } else {
                            eprintln!("  Warning: PATH environment variable is not set!");
                        }
                    }
                    io::ErrorKind::PermissionDenied => {
                        eprintln!("  Hint: Permission denied");
                        eprintln!("  Possible reasons:");
                        eprintln!("    - File is not executable");
                        eprintln!("    - Landlock isolation blocking access");
                        eprintln!("    - Insufficient permissions");
                    }
                    _ => {
                        eprintln!("  Error type: {:?}", e.kind());
                    }
                }

                127 // Standard "command not found" exit code
            }
        };

        // Exit child process with command's exit code
        std::process::exit(status);
    }

    /// Execute a command in interactive mode WITHOUT filesystem isolation (legacy)
    /// This is kept for backwards compatibility but not recommended
    #[allow(dead_code)]
    pub fn execute_interactive_no_isolation(cmd: &ParsedCommand, permissions: &PermissionManager, custom_env: &HashMap<String, String>) -> io::Result<i32> {
        let mut command = Command::new(&cmd.command);
        command.args(&cmd.args);

        // Clear all environment variables and only add allowed ones
        command.env_clear();

        // Add custom env vars first
        for (key, value) in custom_env {
            command.env(key, value);
        }

        // Add system env vars (permissions respected)
        for (key, value) in permissions.get_allowed_env_vars() {
            // Only add if not redacted and not already in custom_env
            if !value.starts_with("[REDACTED") && !custom_env.contains_key(&key) {
                command.env(key, value);
            }
        }

        let status = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(status.code().unwrap_or(-1))
    }
}
