/// Built-in shell commands

use super::parser::ParsedCommand;
use crate::config::Config;
use crate::security::PermissionManager;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinCommand {
    Exit,
    Clear,
    Help,
    Env,
    Allow(String),      // Allow env var access
    Deny(String),       // Deny env var access
    AllowAll,           // Allow all env vars
    DenyAll,            // Deny all env vars
    SecurityStatus,     // Show security status
    Export(String, String), // Set env var: export KEY=VALUE
    Echo(Vec<String>),  // Echo with variable expansion
    AllowPath(String),  // Allow filesystem path access
    DenyPath(String),   // Deny filesystem path access
    ListAllowedPaths,   // List all allowed paths
}

pub struct Builtins;

impl Builtins {
    /// Check if a command is a built-in
    pub fn parse(cmd: &ParsedCommand) -> Option<BuiltinCommand> {
        match cmd.command.as_str() {
            "exit" | "quit" => Some(BuiltinCommand::Exit),
            "clear" | "cls" => Some(BuiltinCommand::Clear),
            "help" => Some(BuiltinCommand::Help),
            "env" => Some(BuiltinCommand::Env),
            "allow" => {
                if cmd.args.is_empty() {
                    Some(BuiltinCommand::AllowAll)
                } else {
                    Some(BuiltinCommand::Allow(cmd.args[0].clone()))
                }
            }
            "deny" => {
                if cmd.args.is_empty() {
                    Some(BuiltinCommand::DenyAll)
                } else {
                    Some(BuiltinCommand::Deny(cmd.args[0].clone()))
                }
            }
            "security" | "status" => Some(BuiltinCommand::SecurityStatus),
            "export" => {
                // Parse export KEY=VALUE with proper quote handling
                // Supports: export KEY="value  with   multiple  spaces"
                //           export KEY='value  with   multiple  spaces'
                // Use raw_input to preserve all spaces
                let raw = &cmd.raw_input;

                // Find first space to skip "export" and any spaces after it
                if let Some(first_space) = raw.find(char::is_whitespace) {
                    // Skip past "export" and all spaces after it until non-space
                    let after_export = &raw[first_space..];
                    let export_arg = after_export.trim_start();

                    if let Some(pos) = export_arg.find('=') {
                        let key = export_arg[..pos].trim().to_string();
                        let value_part = &export_arg[pos + 1..];

                        // Handle quoted values - preserve all spaces within quotes
                        let value = if (value_part.starts_with('"') && value_part.ends_with('"'))
                            || (value_part.starts_with('\'') && value_part.ends_with('\''))
                        {
                            // Remove surrounding quotes, preserve internal spacing
                            value_part[1..value_part.len() - 1].to_string()
                        } else {
                            value_part.to_string()
                        };

                        return Some(BuiltinCommand::Export(key, value));
                    }
                }
                None
            }
            "echo" => Some(BuiltinCommand::Echo(cmd.args.clone())),
            "allowpath" => {
                if cmd.args.is_empty() {
                    None
                } else {
                    Some(BuiltinCommand::AllowPath(cmd.args[0].clone()))
                }
            }
            "denypath" => {
                if cmd.args.is_empty() {
                    None
                } else {
                    Some(BuiltinCommand::DenyPath(cmd.args[0].clone()))
                }
            }
            "listallowpath" | "listpaths" => Some(BuiltinCommand::ListAllowedPaths),
            _ => None,
        }
    }

    /// Execute a built-in command and return output
    /// Note: Commands that modify permissions return a marker that needs to be handled by the caller
    pub fn execute(builtin: &BuiltinCommand, permissions: &PermissionManager, custom_env: &HashMap<String, String>, config: &Config) -> Vec<String> {
        match builtin {
            BuiltinCommand::Exit => vec![],
            BuiltinCommand::Clear => vec![],
            BuiltinCommand::Help => vec![
                "dshell - A Secure Shell written in Rust".to_string(),
                "".to_string(),
                "Built-in commands:".to_string(),
                "  exit, quit       - Exit the shell".to_string(),
                "  clear, cls       - Clear the screen".to_string(),
                "  help             - Show this help message".to_string(),
                "  env              - List environment variables (respects permissions)".to_string(),
                "  export KEY=VALUE - Set environment variable".to_string(),
                "  echo [args]      - Echo arguments (supports $VAR expansion)".to_string(),
                "".to_string(),
                "Security commands:".to_string(),
                "  allow <VAR>           - Allow access to specific env variable".to_string(),
                "  allow                 - Allow access to all env variables".to_string(),
                "  deny <VAR>            - Deny access to specific env variable".to_string(),
                "  deny                  - Deny access to all env variables".to_string(),
                "  allowpath <PATH>      - Allow filesystem access to specific path".to_string(),
                "  denypath <PATH>       - Deny filesystem access to specific path".to_string(),
                "  listallowpath         - List all allowed filesystem paths".to_string(),
                "  security, status      - Show current security status".to_string(),
                "".to_string(),
                "All other commands are executed as external programs.".to_string(),
            ],
            BuiltinCommand::Env => {
                let mut output = vec!["Environment Variables:".to_string(), "".to_string()];

                // Get filtered system environment variables based on permissions
                let env_vars = permissions.get_allowed_env_vars();

                // Format and add system env vars
                for (key, value) in &env_vars {
                    output.push(format!("{}={}", key, value));
                }

                // Add custom environment variables at the end
                if !custom_env.is_empty() {
                    output.push("".to_string());
                    output.push("Custom Environment Variables:".to_string());
                    output.push("".to_string());

                    let mut custom_vars: Vec<(&String, &String)> = custom_env.iter().collect();
                    custom_vars.sort_by_key(|(k, _)| *k);

                    for (key, value) in custom_vars {
                        output.push(format!("{}={}", key, value));
                    }
                }

                output.push("".to_string());
                output.push(format!(
                    "Showing: {} system + {} custom = {} total environment variables",
                    env_vars.len(),
                    custom_env.len(),
                    env_vars.len() + custom_env.len()
                ));

                output
            }
            BuiltinCommand::SecurityStatus => {
                permissions.get_status()
            }
            // These commands need to be handled by the caller to modify permissions
            BuiltinCommand::Allow(var) => {
                vec![format!("PERMISSION_CHANGE:ALLOW:{}", var)]
            }
            BuiltinCommand::Deny(var) => {
                vec![format!("PERMISSION_CHANGE:DENY:{}", var)]
            }
            BuiltinCommand::AllowAll => {
                vec!["PERMISSION_CHANGE:ALLOW_ALL".to_string()]
            }
            BuiltinCommand::DenyAll => {
                vec!["PERMISSION_CHANGE:DENY_ALL".to_string()]
            }
            BuiltinCommand::Export(key, value) => {
                vec![format!("ENV_SET:{}={}", key, value)]
            }
            BuiltinCommand::Echo(args) => {
                // Expand variables in arguments
                let expanded: Vec<String> = args
                    .iter()
                    .map(|arg| Self::expand_variables(arg, permissions, custom_env))
                    .collect();

                vec![expanded.join(" ")]
            }
            BuiltinCommand::AllowPath(path) => {
                vec![format!("PERMISSION_CHANGE:ALLOW_PATH:{}", path)]
            }
            BuiltinCommand::DenyPath(path) => {
                vec![format!("PERMISSION_CHANGE:DENY_PATH:{}", path)]
            }
            BuiltinCommand::ListAllowedPaths => {
                let mut output = vec!["Allowed Filesystem Paths:".to_string(), "".to_string()];

                // Show paths from config file
                output.push("From config file:".to_string());
                if config.additional_allowed_paths.is_empty() {
                    output.push("  (none)".to_string());
                } else {
                    for path in &config.additional_allowed_paths {
                        output.push(format!("  ✓ {}", path));
                    }
                }
                output.push("".to_string());

                // Show dynamically added paths (from allowpath command)
                let dynamic_paths = permissions.list_allowed_paths();
                output.push("Dynamically added (via allowpath command):".to_string());
                if dynamic_paths.is_empty() {
                    output.push("  (none)".to_string());
                } else {
                    for path in dynamic_paths {
                        output.push(format!("  ✓ {}", path));
                    }
                }

                output.push("".to_string());
                let total = config.additional_allowed_paths.len() + permissions.list_allowed_paths().len();
                output.push(format!("Total: {} allowed path(s) ({} from config + {} dynamic)",
                    total,
                    config.additional_allowed_paths.len(),
                    permissions.list_allowed_paths().len()
                ));

                output
            }
        }
    }

    /// Expand $VAR and ${VAR} in a string
    fn expand_variables(s: &str, permissions: &PermissionManager, custom_env: &HashMap<String, String>) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    // ${VAR} syntax
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }
                    result.push_str(&Self::get_var_value(&var_name, permissions, custom_env));
                } else {
                    // $VAR syntax
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    result.push_str(&Self::get_var_value(&var_name, permissions, custom_env));
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Get variable value from custom env or system env (respecting permissions)
    fn get_var_value(var_name: &str, permissions: &PermissionManager, custom_env: &HashMap<String, String>) -> String {
        // Check custom env first
        if let Some(value) = custom_env.get(var_name) {
            return value.clone();
        }

        // Check system env with permissions
        let env_vars = permissions.get_allowed_env_vars();
        for (key, value) in env_vars {
            if key == var_name && !value.starts_with("[REDACTED") {
                return value;
            }
        }

        // Variable not found or not allowed
        String::new()
    }
}
