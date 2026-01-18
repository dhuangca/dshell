mod config;
mod security;
mod shell;
mod terminal;

use config::{Config, PROMPT};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self as crossterm_terminal, ClearType},
    tty::IsTty,
};
use security::{Permission, PermissionManager, LandlockIsolation};
use shell::{executor::Executor, CommandAction, Shell};
use std::collections::HashMap;
use std::io;
use terminal::{InputAction, InputEditor, TerminalRenderer};

struct App {
    renderer: TerminalRenderer,
    input_editor: InputEditor,
    permissions: PermissionManager,
    custom_env: HashMap<String, String>,
    config: Config,
}

impl App {
    fn new() -> Self {
        let mut permissions = PermissionManager::new();

        // Load configuration from ~/.config/dshell/config.toml
        let config = Config::load();

        // Apply denied paths from config to PermissionManager
        for path in &config.denied_paths {
            permissions.deny_path(path.clone());
        }

        // Add startup message about security
        let mut renderer = TerminalRenderer::new(config::WELCOME_MESSAGE.to_string());
        renderer.add_output("".to_string());
        renderer.add_output("🔒 Security Features:".to_string());
        renderer.add_output("".to_string());

        // Environment variable security
        renderer.add_output("  • Environment Variables: Filtered by default".to_string());
        renderer.add_output("    Allowed: HOME, PATH, USER, SHELL, TERM, LANG, EDITOR, COLORTERM".to_string());
        renderer.add_output("    Plus Rust toolchain vars: RUSTUP_HOME, CARGO_HOME, etc.".to_string());
        renderer.add_output("    Use 'allow <VAR>' or 'deny <VAR>' to modify permissions".to_string());
        renderer.add_output("".to_string());

        // Filesystem isolation status
        if LandlockIsolation::is_available() {
            renderer.add_output("  • Filesystem Isolation: ENABLED (Landlock)".to_string());
            if let Some(abi) = LandlockIsolation::get_abi_version() {
                renderer.add_output(format!("    Landlock ABI version: {:?}", abi).to_string());
            }
            renderer.add_output("    Interactive commands restricted to current directory".to_string());
            renderer.add_output("    ✓ Kernel-enforced - cannot be bypassed".to_string());
        } else {
            renderer.add_output("  • Filesystem Isolation: NOT AVAILABLE".to_string());
            renderer.add_output("    Requires Linux kernel 5.13+ with Landlock support".to_string());
            renderer.add_output("    ⚠️  Commands can access parent directories".to_string());
        }
        renderer.add_output("".to_string());

        // Show configuration loaded
        renderer.add_output("📋 Configuration:".to_string());
        renderer.add_output("".to_string());

        // Show interactive commands
        renderer.add_output(format!("  • Interactive Commands ({})", config.interactive_commands.len()));
        let cmd_list = config.interactive_commands.join(", ");
        let max_width = 70;
        if cmd_list.len() <= max_width {
            renderer.add_output(format!("    {}", cmd_list));
        } else {
            // Split into multiple lines if too long
            let mut current_line = String::new();
            for (i, cmd) in config.interactive_commands.iter().enumerate() {
                if current_line.len() + cmd.len() + 2 > max_width {
                    renderer.add_output(format!("    {}", current_line));
                    current_line = cmd.clone();
                } else {
                    if !current_line.is_empty() {
                        current_line.push_str(", ");
                    }
                    current_line.push_str(cmd);
                }
                if i == config.interactive_commands.len() - 1 && !current_line.is_empty() {
                    renderer.add_output(format!("    {}", current_line));
                }
            }
        }
        renderer.add_output("".to_string());

        // Show additional allowed paths
        renderer.add_output(format!("  • Additional Allowed Paths ({})", config.additional_allowed_paths.len()));
        if config.additional_allowed_paths.is_empty() {
            renderer.add_output("    (none)".to_string());
        } else {
            for path in &config.additional_allowed_paths {
                renderer.add_output(format!("    {}", path));
            }
        }
        renderer.add_output("".to_string());

        // Show denied paths if any are configured
        if !config.denied_paths.is_empty() {
            renderer.add_output(format!("  • Denied Paths ({})", config.denied_paths.len()));
            for path in &config.denied_paths {
                renderer.add_output(format!("    ✗ {}", path));
            }
            renderer.add_output("".to_string());
        }

        renderer.add_output("Type 'help' for commands, 'security' for status".to_string());
        renderer.add_output("".to_string());

        App {
            renderer,
            input_editor: InputEditor::new(),
            permissions,
            custom_env: HashMap::new(),
            config,
        }
    }

    fn run(&mut self) -> io::Result<()> {
        // Check if stdin is a TTY
        if !io::stdin().is_tty() {
            // Non-interactive mode: read from stdin
            return self.run_non_interactive();
        }

        // Enter raw mode
        crossterm_terminal::enable_raw_mode()?;

        // Initial render
        self.render()?;

        let result = self.event_loop();

        // Exit raw mode
        crossterm_terminal::disable_raw_mode()?;

        // Clear screen on exit
        execute!(io::stdout(), crossterm_terminal::Clear(ClearType::All))?;
        execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0))?;

        result
    }

    fn run_non_interactive(&mut self) -> io::Result<()> {
        use std::io::BufRead;

        // Print initial output (welcome message, etc.)
        for line in self.renderer.get_new_output() {
            println!("{}", line);
        }

        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let command = line?;

            // Skip empty lines
            if command.trim().is_empty() {
                continue;
            }

            // Process command
            if !self.handle_command(&command)? {
                break;
            }

            // Print new output
            for line in self.renderer.get_new_output() {
                println!("{}", line);
            }
        }

        Ok(())
    }

    fn event_loop(&mut self) -> io::Result<()> {
        loop {
            // Read event
            if let Event::Key(key_event) = event::read()? {
                match self.input_editor.handle_key(key_event) {
                    InputAction::Exit => break,
                    InputAction::Submit(command) => {
                        if !self.handle_command(&command)? {
                            break;
                        }
                    }
                    InputAction::None => {}
                }

                self.render()?;
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, command: &str) -> io::Result<bool> {
        // Add command to output
        self.renderer
            .add_output(format!("{}{}", PROMPT, command));

        // Add to history
        self.input_editor.add_to_history(command.to_string());

        // Clear input
        self.input_editor.clear();

        // Process the command
        if let Some(action) = Shell::process_input(command, &self.permissions, &self.custom_env, &self.config) {
            match action {
                CommandAction::Exit => return Ok(false),
                CommandAction::ClearScreen => {
                    self.renderer.clear_output();
                }
                CommandAction::ShowOutput(lines) => {
                    self.renderer.add_output_lines(lines);
                }
                CommandAction::ExecuteCaptured(cmd) => {
                    let output = Shell::execute_captured(&cmd, &self.permissions, &self.custom_env);
                    self.renderer.add_output_lines(output);
                }
                CommandAction::ExecuteInteractive(cmd) => {
                    // Disable raw mode and clear screen
                    crossterm_terminal::disable_raw_mode()?;
                    TerminalRenderer::clear_screen()?;

                    // Execute the command with Landlock filesystem isolation
                    match Executor::execute_interactive(&cmd, &self.permissions, &self.custom_env, &self.config) {
                        Ok(_status) => {
                            // Command executed successfully
                            // The isolation status is printed by the executor
                        }
                        Err(e) => {
                            eprintln!("dshell: {}: {}", cmd.command, e);
                        }
                    }

                    // Re-enable raw mode
                    crossterm_terminal::enable_raw_mode()?;

                    // Add continuation message
                    self.renderer
                        .add_output("[Nothing to display. Press Enter to continue]".to_string());
                }
                CommandAction::AllowEnvVar(var) => {
                    self.permissions.allow_env_var(var.clone());
                    self.renderer
                        .add_output(format!("✓ Allowed access to: {}", var));
                }
                CommandAction::DenyEnvVar(var) => {
                    self.permissions.deny_env_var(var.clone());
                    self.renderer
                        .add_output(format!("✗ Denied access to: {}", var));
                }
                CommandAction::AllowAllEnvVars => {
                    self.permissions.set_env_access(Permission::Allowed);
                    self.renderer
                        .add_output("✓ Allowed access to ALL environment variables".to_string());
                }
                CommandAction::DenyAllEnvVars => {
                    self.permissions.set_env_access(Permission::Denied);
                    self.renderer
                        .add_output("✗ Denied access to ALL environment variables".to_string());
                }
                CommandAction::SetEnvVar(key, value) => {
                    self.custom_env.insert(key.clone(), value.clone());
                    self.renderer
                        .add_output(format!("✓ Set environment variable: {}={}", key, value));
                }
                CommandAction::AllowPath(path) => {
                    self.permissions.allow_path(path.clone());
                    self.renderer
                        .add_output(format!("✓ Allowed filesystem access to: {}", path));
                }
                CommandAction::DenyPath(path) => {
                    self.permissions.deny_path(path.clone());
                    self.renderer
                        .add_output(format!("✗ Denied filesystem access to: {}", path));
                }
            }
        }

        Ok(true)
    }

    fn render(&self) -> io::Result<()> {
        self.renderer
            .render(self.input_editor.buffer(), self.input_editor.cursor_pos())
    }
}

fn main() -> io::Result<()> {
    let mut app = App::new();
    app.run().map_err(|e| {
        eprintln!("Error: {}", e);
        e
    })
}
