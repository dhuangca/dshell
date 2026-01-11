# Changelog

All notable changes to dshell will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-11

### Added
- **Rust toolchain support**: Added RUSTUP_HOME, CARGO_HOME, RUST_BACKTRACE, RUSTC, and RUSTDOC environment variables to the default allowed list
- **Config file denied paths**: Added `denied_paths` configuration option in `config.toml` to block access to specific directories
- **TTY detection**: Added proper terminal detection with clear error messages when run without a TTY
- **Clipboard paste support**: Added Ctrl+V and Ctrl+Shift+V to paste from clipboard
- **Comprehensive documentation**: Added README.md, INSTALL.md, and config.toml.example

### Changed
- Updated startup message to show Rust environment variable support
- Improved error messages when shell is run in non-interactive mode
- Denied paths now take precedence over allowed paths for better security
- Configuration now shows denied paths on startup when configured

### Fixed
- Fixed "No such device or address" error when shell is run without a TTY - now shows helpful error message
- Fixed Rust commands (cargo, rustc) not working due to missing environment variables

## [0.1.0] - 2024-12-27

### Added
- Initial release with Landlock filesystem isolation
- Environment variable filtering
- Dynamic path permission commands (allowpath, denypath)
- Interactive command support with filesystem restrictions
- Configuration file support (~/.config/dshell/config.toml)
- Built-in commands: help, env, security, allow, deny, export, echo
- Clipboard integration
- Command history with up/down arrow navigation

### Security
- Kernel-enforced filesystem isolation using Linux Landlock LSM
- Default whitelist of safe environment variables
- Per-path access control
- Protection against directory traversal outside working directory
