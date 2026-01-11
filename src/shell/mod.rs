/// Shell module - Core shell functionality

pub mod builtins;
pub mod executor;
pub mod parser;

use builtins::{BuiltinCommand, Builtins};
use crate::config::Config;
use crate::security::PermissionManager;
use executor::{ExecutionMode, Executor};
use parser::ParsedCommand;
use std::collections::HashMap;

#[derive(Debug)]
pub enum CommandAction {
    Exit,
    ClearScreen,
    ExecuteInteractive(ParsedCommand),
    ExecuteCaptured(ParsedCommand),
    ShowOutput(Vec<String>),
    AllowEnvVar(String),
    DenyEnvVar(String),
    AllowAllEnvVars,
    DenyAllEnvVars,
    SetEnvVar(String, String),
    AllowPath(String),
    DenyPath(String),
}

pub struct Shell;

impl Shell {
    /// Process a command input and determine what action to take
    pub fn process_input(input: &str, permissions: &PermissionManager, custom_env: &HashMap<String, String>, config: &Config) -> Option<CommandAction> {
        let cmd = ParsedCommand::parse(input)?;

        // Check if it's a built-in command
        if let Some(builtin) = Builtins::parse(&cmd) {
            return Some(match builtin {
                BuiltinCommand::Exit => CommandAction::Exit,
                BuiltinCommand::Clear => CommandAction::ClearScreen,
                BuiltinCommand::Help
                | BuiltinCommand::Env
                | BuiltinCommand::SecurityStatus
                | BuiltinCommand::Echo(_)
                | BuiltinCommand::ListAllowedPaths => {
                    let output = Builtins::execute(&builtin, permissions, custom_env, config);
                    CommandAction::ShowOutput(output)
                }
                BuiltinCommand::Allow(var) => CommandAction::AllowEnvVar(var),
                BuiltinCommand::Deny(var) => CommandAction::DenyEnvVar(var),
                BuiltinCommand::AllowAll => CommandAction::AllowAllEnvVars,
                BuiltinCommand::DenyAll => CommandAction::DenyAllEnvVars,
                BuiltinCommand::Export(key, value) => CommandAction::SetEnvVar(key, value),
                BuiltinCommand::AllowPath(path) => CommandAction::AllowPath(path),
                BuiltinCommand::DenyPath(path) => CommandAction::DenyPath(path),
            });
        }

        // Check execution mode
        match Executor::execution_mode(&cmd, config) {
            ExecutionMode::Interactive => Some(CommandAction::ExecuteInteractive(cmd)),
            ExecutionMode::Captured => Some(CommandAction::ExecuteCaptured(cmd)),
        }
    }

    /// Execute a captured command with environment filtering
    pub fn execute_captured(cmd: &ParsedCommand, permissions: &PermissionManager, custom_env: &HashMap<String, String>) -> Vec<String> {
        let result = Executor::execute_captured(cmd, permissions, custom_env);
        result.output
    }
}
