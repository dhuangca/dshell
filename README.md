# dshell - Secure Shell with Filesystem Isolation

A security-focused shell that uses **Linux Landlock** to provide kernel-enforced filesystem isolation for interactive commands like Claude Code, vim, and other tools.

## What It Does

🔒 **Restricts commands to current directory** - Interactive commands can only access files in your working directory

✅ **Kernel-enforced** - Uses Linux Landlock LSM, cannot be bypassed

🔑 **Environment filtering** - Controls which environment variables commands can see

⚙️ **Configurable** - Easy to add custom allowed paths for tools like Claude

## Quick Start

```bash
# Build
cargo build --release

# Run
cd ~/my-project
/work/oor/shell/target/release/dshell

# Use Claude safely - it can only access ~/my-project
dshell> claude
```

## Features

- **Filesystem Isolation (Landlock)** - Restricts interactive commands to current directory
- **Environment Variable Filtering** - Controls access to sensitive environment variables
- **Configurable Paths** - Add custom paths for tools that need config access
- **Pre-configured for Claude** - Works with Claude Code out of the box
- **Good Error Messages** - Clear feedback when things go wrong
- **Graceful Fallback** - Works on older systems without Landlock (with warnings)

## Example Use Case

```bash
# You're working on a sensitive project
cd ~/work/sensitive-project

# Run dshell
dshell

# Run Claude - it's isolated to this project only
dshell> claude

# Claude can access:
# ✅ ~/work/sensitive-project/* (current directory)
# ✅ ~/.claude/ (its config)
# ✅ System binaries (/usr/bin, etc.)

# Claude CANNOT access:
# ❌ ~/work/other-project/ (different project)
# ❌ ~/Documents/ (outside project)
# ❌ ../parent/ (parent directories)
```

## Requirements

- **OS:** Linux
- **Kernel:** 5.13+ (for Landlock) - graceful fallback on older kernels
- **Build:** Rust 1.70+
- **Runtime:** Regular user, no root needed

## Documentation

📖 **[INSTALL.md](INSTALL.md)** - Installation and usage guide
📖 **[USAGE.md](USAGE.md)** - How to use dshell
📖 **[CONFIGURING_PATHS.md](CONFIGURING_PATHS.md)** - How to add custom paths
📖 **[FILESYSTEM_ISOLATION_PLAN.md](FILESYSTEM_ISOLATION_PLAN.md)** - Technical details

## Installation

### Quick Install (User)

```bash
cargo build --release
cp target/release/dshell ~/.local/bin/
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### System-wide Install

```bash
cargo build --release
sudo cp target/release/dshell /usr/local/bin/
```

See [INSTALL.md](INSTALL.md) for more options.

## Configuration

Configuration is loaded from `~/.config/dshell/config.toml`. A template is provided at `config.toml.example`.

```bash
# Copy the example config
cp config.toml.example ~/.config/dshell/config.toml

# Edit to add your own paths
nano ~/.config/dshell/config.toml
```

Example configuration:

```toml
# Interactive commands that run with filesystem isolation
interactive_commands = ["claude", "vim", "bash", "python", ...]

# Additional paths allowed for isolated commands
additional_allowed_paths = [
    "~/.claude",           # Claude config
    "~/.cargo",            # Rust toolchain
    "~/.rustup",           # Rust toolchain manager
    "~/.nvm",              # Node.js
    # Add your own paths here
]
```

See [CONFIGURING_PATHS.md](CONFIGURING_PATHS.md) for details.

## How It Works

### Isolation Levels

**Without Landlock (Linux <5.13):**
- ⚠️ Environment variables filtered
- ⚠️ Working directory set
- ❌ NO filesystem isolation

**With Landlock (Linux 5.13+):**
- ✅ Environment variables filtered
- ✅ Working directory set
- ✅ **Filesystem isolated by kernel**

### What Gets Isolated

**Interactive commands** (configured in `src/config.rs`):
- `claude`, `vim`, `nvim`, `nano`, `emacs`
- `bash`, `sh`, `python`, `node`
- `less`, `more`, `man`, `ssh`

**Regular commands** (NOT isolated):
- `cat`, `ls`, `grep`, `find`, `cp`, `mv`
- Standard utilities

## Security

### ✅ What It Protects Against

- Prevents Claude/tools from accessing files outside current directory
- Prevents reading `../sensitive-data/secrets.txt`
- Prevents writing to `/etc/` or other system directories
- Blocks access to other projects or home directory files
- Filters sensitive environment variables

### ⚠️ Limitations

- **Linux-only** (Landlock doesn't exist on macOS/Windows)
- **Requires kernel 5.13+** for full isolation
- **Can't restrict network access** (tools can still make API calls)
- **System paths accessible** (needed for binaries to run)

See technical details in [FILESYSTEM_ISOLATION_PLAN.md](FILESYSTEM_ISOLATION_PLAN.md).

## Built-in Commands

```bash
help              # Show help
env               # List environment variables
security          # Show security status
allow <VAR>       # Allow environment variable
deny <VAR>        # Deny environment variable
export KEY=VALUE  # Set environment variable
echo $VAR         # Echo with variable expansion
exit              # Exit dshell
```

## Example Session

```bash
$ cd ~/my-app
$ dshell

Welcome to dshell terminal!

🔒 Security Features:

  • Environment Variables: Filtered by default
  • Filesystem Isolation: ENABLED (Landlock)
    Landlock ABI version: V2
    ✓ Kernel-enforced - cannot be bypassed

dshell> security
Security Status:
  Environment Access: Selective
  Allowed: HOME, PATH, USER, SHELL, TERM, LANG, EDITOR, COLORTERM
  Plus Rust toolchain vars: RUSTUP_HOME, CARGO_HOME, etc.

dshell> claude
🔒 Filesystem isolated to: /home/user/my-app
# Claude starts and can only access /home/user/my-app

dshell> exit
$
```

## Claude Code Configuration

If you need to configure Claude Code to use a specific HOME directory, add the following to your Claude Code settings:

**File location:** `.claude/settings.json` (in your project) or `~/.claude/settings.json` (global)

```json
{
  "env": {
    "HOME": "/home/dandan"
  }
}
```

This ensures Claude Code uses the correct HOME path for all commands. See [Claude Code settings documentation](https://code.claude.com/docs/en/settings) for more details.

## Troubleshooting

### Build fails with "linker `cc` not found"?

If you encounter linker errors during build, you may need to configure cargo to use your system's GCC linker explicitly.

Create `.cargo/config.toml` in your project directory:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "/usr/bin/gcc"
```

**Note:** This configuration is **target-specific** and only affects builds for `x86_64-unknown-linux-gnu` (64-bit Linux). It will not impact builds for other platforms like Windows, macOS, or ARM architectures. Each platform uses its own default linker.

For CI/CD or temporary builds, you can also use an environment variable:
```bash
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/usr/bin/gcc cargo build
```

Or ensure `/usr/bin` is in your PATH when building:
```bash
PATH="/usr/bin:$PATH" cargo build
```

### Landlock not available?

Check your kernel:
```bash
uname -r  # Need 5.13+
cat /sys/kernel/security/lsm | grep landlock
```

### Command fails?

Check the error message - it will tell you if:
- Command not found (not in PATH)
- Permission denied (Landlock blocking it)
- Missing environment variable

### Claude can't access API?

Make sure these are in `additional_allowed_paths` in `~/.config/dshell/config.toml`:
- `~/.claude`
- `~/.claude.json`
- `~/.nvm`
- `~/.npm`

No rebuild needed - just restart dshell.

### Rust commands (cargo, rustc) not working?

The shell now includes Rust environment variables by default (RUSTUP_HOME, CARGO_HOME, etc.).

Make sure these paths are in `additional_allowed_paths` in `~/.config/dshell/config.toml`:
- `~/.cargo`
- `~/.rustup`

These are included in the default config template.

If cargo is still not found, ensure `~/.cargo/bin` is in your PATH:
```bash
echo $PATH | grep cargo
```

## Contributing

This was built to safely run Claude Code and similar AI coding assistants in isolated environments.

Ideas for improvements:
- Network isolation (Landlock ABI V4+)
- Dynamic path detection
- Configuration file instead of recompiling
- More granular permissions (read-only vs read-write)

## License

[Add your license here]

## Acknowledgments

Built with:
- **Landlock** - Linux kernel LSM for filesystem isolation
- **crossterm** - Terminal handling
- **Rust** - Systems programming language

Generated with assistance from [Claude Code](https://claude.com/claude-code) 🤖
