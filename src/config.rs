/// Configuration and constants for the shell

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Shell prompt text
pub const PROMPT: &str = "dshell> ";

/// Welcome message displayed on startup
pub const WELCOME_MESSAGE: &str = "Welcome to dshell terminal!";

/// Maximum command history size
pub const MAX_HISTORY_SIZE: usize = 1000;

/// Default interactive commands
const DEFAULT_INTERACTIVE_COMMANDS: &[&str] = &[
    "claude",
    "ollama",
    "vim",
    "nvim",
    "nano",
    "emacs",
    "less",
    "more",
    "top",
    "htop",
    "man",
    "python",
    "node",
    "irb",
    "ssh",
    "bash",
    "sh",
    "git",
    "kubectl"
];

/// Default additional allowed paths
const DEFAULT_ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    // Claude Code configuration and data
    "~/.claude",           // Claude config directory
    "~/.claude.json",      // Claude config file
    "~/.nvm",              // Node Version Manager (needed for Claude binary)
    "~/.npm",              // npm cache and config
    "/dev/null",
    "~/.cargo",            // Rust package manager (needed for building Claude binary)
    "~/.local/bin",        // Local user binaries
    "~/.rustup",          // Rust toolchain manager
];

/// Default denied paths (empty by default - user must explicitly configure)
const DEFAULT_DENIED_PATHS: &[&str] = &[];

/// User configuration loaded from ~/.config/dshell/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// List of interactive commands that need direct terminal access
    #[serde(default = "default_interactive_commands")]
    pub interactive_commands: Vec<String>,

    /// Additional paths to allow access to for isolated commands
    #[serde(default = "default_additional_allowed_paths")]
    pub additional_allowed_paths: Vec<String>,

    /// Paths to explicitly deny access to for isolated commands
    /// These take precedence over allowed paths
    #[serde(default = "default_denied_paths")]
    pub denied_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interactive_commands: default_interactive_commands(),
            additional_allowed_paths: default_additional_allowed_paths(),
            denied_paths: default_denied_paths(),
        }
    }
}

impl Config {
    /// Get the path to the config file
    pub fn config_path() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".config/dshell/config.toml"))
    }

    /// Load configuration from ~/.config/dshell/config.toml
    /// Falls back to defaults if file doesn't exist or can't be parsed
    pub fn load() -> Self {
        if let Some(config_path) = Self::config_path() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                match toml::from_str::<Config>(&content) {
                    Ok(config) => {
                        eprintln!("✓ Loaded configuration from: {}", config_path.display());
                        return config;
                    }
                    Err(e) => {
                        eprintln!("⚠ Failed to parse config file {}: {}", config_path.display(), e);
                        eprintln!("  Using default configuration");
                    }
                }
            }
        }

        // Return default config if file doesn't exist or can't be loaded
        Self::default()
    }

    /// Save the current configuration to ~/.config/dshell/config.toml
    #[allow(dead_code)]
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(config_path) = Self::config_path() {
            // Create parent directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let toml_string = toml::to_string_pretty(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            fs::write(&config_path, toml_string)?;
            eprintln!("✓ Configuration saved to: {}", config_path.display());
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config path (HOME not set)",
            ))
        }
    }

    /// Create a default config file with helpful comments
    #[allow(dead_code)]
    pub fn create_default_config_file() -> std::io::Result<()> {
        if let Some(config_path) = Self::config_path() {
            // Create parent directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let default_config = r#"# dshell Configuration File
# This file is located at ~/.config/dshell/config.toml

# List of interactive commands that need direct terminal access
# These commands will run with full terminal control (stdin/stdout/stderr)
interactive_commands = [
    "claude",
    "ollama",
    "vim",
    "nvim",
    "nano",
    "emacs",
    "less",
    "more",
    "top",
    "htop",
    "man",
    "python",
    "node",
    "irb",
    "ssh",
    "bash",
    "sh",
]

# Additional paths to allow access to for isolated commands
# These paths will have read-write access even when Landlock isolation is enabled
# Use this to grant access to config files, tools, or data directories
additional_allowed_paths = [
    # Claude Code configuration and data
    "~/.claude",           # Claude config directory
    "~/.claude.json",      # Claude config file
    "~/.nvm",              # Node Version Manager (needed for Claude binary)
    "~/.npm",              # npm cache and config
    "/dev/null",
    "~/.cargo",            # Rust package manager
    "~/.local/bin",        # Local user binaries
    "~/.rustup",           # Rust toolchain manager

    # Add your own paths here, for example:
    # "~/.config/myapp",
    # "~/.local/share/myapp",
]

# Paths to explicitly deny access to for isolated commands
# These take precedence over allowed paths
# Use this to block access to sensitive directories
denied_paths = [
    # Examples (uncomment to use):
    # "~/.ssh",              # SSH keys
    # "~/.gnupg",            # GPG keys
    # "~/.aws",              # AWS credentials
    # "~/.config/gcloud",    # Google Cloud credentials
    # "~/sensitive-data",    # Your sensitive files
]
"#;

            fs::write(&config_path, default_config)?;
            eprintln!("✓ Created default configuration file: {}", config_path.display());
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config path (HOME not set)",
            ))
        }
    }
}

fn default_interactive_commands() -> Vec<String> {
    DEFAULT_INTERACTIVE_COMMANDS.iter().map(|s| s.to_string()).collect()
}

fn default_additional_allowed_paths() -> Vec<String> {
    DEFAULT_ADDITIONAL_ALLOWED_PATHS.iter().map(|s| s.to_string()).collect()
}

fn default_denied_paths() -> Vec<String> {
    DEFAULT_DENIED_PATHS.iter().map(|s| s.to_string()).collect()
}
