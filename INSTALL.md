# Installation and Usage Guide

## Quick Start

### 1. Build the Release Binary

```bash
cd /work/oor/shell
cargo build --release
```

The optimized binary will be at: `/work/oor/shell/target/release/dshell`

### 2. Test It

```bash
# Navigate to a test directory
cd /tmp/test-project

# Run dshell
/work/oor/shell/target/release/dshell
```

You should see:
```
Welcome to dshell terminal!

🔒 Security Features:

  • Environment Variables: Filtered by default
  • Filesystem Isolation: ENABLED (Landlock)
    ✓ Kernel-enforced - cannot be bypassed

Type 'help' for commands, 'security' for status

dshell>
```

### 3. Try Some Commands

```bash
dshell> ls
dshell> cat file.txt
dshell> bash
dshell> claude
dshell> exit
```

## Installation Options

### Option 1: Use Directly from Build Directory

**Simplest - No installation needed**

Create an alias in your `~/.bashrc` or `~/.zshrc`:

```bash
alias dshell='/work/oor/shell/target/release/dshell'
```

Then reload your shell:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

Now you can run:
```bash
cd ~/my-project
dshell
```

### Option 2: Install to User's Local Bin

**Recommended for single user**

```bash
# Create local bin directory if it doesn't exist
mkdir -p ~/.local/bin

# Copy the binary
cp /work/oor/shell/target/release/dshell ~/.local/bin/

# Make sure ~/.local/bin is in your PATH
# Add to ~/.bashrc or ~/.zshrc if not already there:
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Now you can run:
```bash
dshell
```

### Option 3: Install System-Wide

**For all users on the system**

```bash
# Requires sudo/root privileges
sudo cp /work/oor/shell/target/release/dshell /usr/local/bin/

# Verify it's installed
which dshell
# Should output: /usr/local/bin/dshell
```

Now any user can run:
```bash
dshell
```

### Option 4: Create a Wrapper Script

**For custom configurations or shortcuts**

Create `~/bin/claude-safe`:

```bash
#!/bin/bash
# Run Claude Code in isolated environment

PROJECT_DIR="${1:-.}"  # Use first arg or current directory

cd "$PROJECT_DIR" || exit 1
echo "🔒 Starting dshell in isolated mode"
echo "   Working directory: $(pwd)"
echo "   Claude will only access files here"
echo ""

exec /work/oor/shell/target/release/dshell
```

Make it executable:
```bash
chmod +x ~/bin/claude-safe
```

Usage:
```bash
claude-safe ~/my-project  # Start in specific project
claude-safe               # Start in current directory
```

## How to Use dshell

### Basic Workflow

1. **Navigate to your project directory:**
   ```bash
   cd ~/my-important-project
   ```

2. **Start dshell:**
   ```bash
   dshell
   ```

3. **Run commands - they're isolated:**
   ```bash
   dshell> claude
   # Claude can only access files in ~/my-important-project
   ```

4. **Exit when done:**
   ```bash
   dshell> exit
   ```

### Running Claude Safely

```bash
# Go to your project
cd ~/work/my-app

# Start dshell
dshell

# Run Claude - it's now restricted to ~/work/my-app
dshell> claude
```

**What Claude can access:**
- ✅ All files in `~/work/my-app/` (and subdirectories)
- ✅ `~/.claude/` (configuration)
- ✅ `~/.nvm/` (Node.js installation)
- ✅ `/tmp/` (temporary files)
- ✅ System binaries (/usr/bin, /bin)

**What Claude CANNOT access:**
- ❌ `~/work/other-app/` (different project)
- ❌ `~/Documents/` (outside project)
- ❌ `../parent-directory/` (parent directories)
- ❌ `/etc/passwd` (system files for writing)

### Built-in Commands

```bash
# Get help
dshell> help

# View environment variables
dshell> env

# Check security status
dshell> security

# Allow/deny specific environment variables
dshell> allow MY_VAR
dshell> deny SECRET_KEY

# Set environment variables
dshell> export MY_VAR="value"

# Echo with variable expansion
dshell> echo $HOME $PATH

# Exit
dshell> exit  # or quit, or Ctrl+D
```

### Environment Variable Management

```bash
# By default, common variables are allowed
dshell> env
# Shows: HOME, PATH, USER, SHELL, TERM, LANG, EDITOR, COLORTERM

# Allow more variables
dshell> allow AWS_PROFILE
dshell> allow GITHUB_TOKEN

# Deny all variables (maximum security)
dshell> deny
dshell> env
# Shows: (empty or very minimal)

# Allow specific variables back
dshell> allow PATH
dshell> allow HOME

# Allow all variables
dshell> allow

# Check what's allowed
dshell> security
```

## Updating/Rebuilding

When you make changes to the code or configuration:

```bash
cd /work/oor/shell

# Rebuild
cargo build --release

# If you installed it, copy the new binary
cp target/release/dshell ~/.local/bin/
# or
sudo cp target/release/dshell /usr/local/bin/
```

## Adding Custom Allowed Paths

If you need Claude or other tools to access additional paths:

### Step 1: Edit Configuration

```bash
vim src/config.rs
```

Find `ADDITIONAL_ALLOWED_PATHS` and add your paths:

```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    "~/.claude",
    "~/.claude.json",
    "~/.nvm",
    "~/.npm",

    // Add your paths here:
    "~/.config/myapp",      // Your app config
    "~/.local/share/data",  // Your data directory
];
```

### Step 2: Rebuild

```bash
cargo build --release
```

### Step 3: Reinstall (if needed)

```bash
cp target/release/dshell ~/.local/bin/
# or
sudo cp target/release/dshell /usr/local/bin/
```

See [CONFIGURING_PATHS.md](CONFIGURING_PATHS.md) for more details.

## Verifying Installation

### Check Version

```bash
dshell --version 2>/dev/null || echo "dshell doesn't have --version yet"
```

### Check Location

```bash
which dshell
# Should show: /usr/local/bin/dshell
# or: /home/username/.local/bin/dshell
```

### Check Landlock Support

Start dshell and look for:
```
• Filesystem Isolation: ENABLED (Landlock)
```

If you see:
```
• Filesystem Isolation: NOT AVAILABLE
```

Then Landlock is not available (older kernel or not enabled).

### Test Isolation

```bash
# Create test environment
mkdir -p /tmp/test-isolation/{workspace,forbidden}
echo "allowed" > /tmp/test-isolation/workspace/allowed.txt
echo "secret" > /tmp/test-isolation/forbidden/secret.txt

# Start dshell from workspace
cd /tmp/test-isolation/workspace
dshell

# Test isolation with bash
dshell> bash
$ cat allowed.txt          # Should work ✅
$ cat ../forbidden/secret.txt  # Should fail ❌ (Permission denied)
$ exit

dshell> exit
```

## Uninstalling

### If installed to ~/.local/bin:
```bash
rm ~/.local/bin/dshell
```

### If installed to /usr/local/bin:
```bash
sudo rm /usr/local/bin/dshell
```

### Remove alias:
Edit `~/.bashrc` or `~/.zshrc` and remove the alias line.

## Troubleshooting

### "Command not found: dshell"

**Solution:**
1. Check if it's in PATH: `which dshell`
2. If not, either:
   - Add to PATH: `export PATH="$HOME/.local/bin:$PATH"`
   - Use full path: `/work/oor/shell/target/release/dshell`
   - Create alias: `alias dshell='/work/oor/shell/target/release/dshell'`

### "Error: No such device or address"

**Cause:** Running dshell without a terminal (piped input).

**Solution:** Run dshell in a real terminal, not from a script with piped input.

### "Filesystem Isolation: NOT AVAILABLE"

**Cause:** Landlock not available on your system.

**Solutions:**
1. Check kernel version: `uname -r` (need 5.13+)
2. Check LSM: `cat /sys/kernel/security/lsm | grep landlock`
3. If kernel is old: Upgrade kernel or use Docker
4. If Landlock not loaded: Enable in kernel config

### Commands fail with "Permission denied"

**Possible causes:**

1. **Landlock is blocking access**
   - Check: `echo $DSHELL_ISOLATION_STATUS`
   - If "fully_enforced": Add needed paths to `ADDITIONAL_ALLOWED_PATHS`

2. **Missing binary**
   - Check: `which <command>`
   - Make sure PATH includes binary location

3. **Environment variable missing**
   - Check: `dshell> env`
   - Allow needed variables: `dshell> allow VAR_NAME`

### Claude can't access API

**Solution:** Make sure these paths are in `ADDITIONAL_ALLOWED_PATHS`:
- `~/.claude`
- `~/.claude.json`
- `~/.nvm`
- `~/.npm`

And rebuild:
```bash
cargo build --release
cp target/release/dshell ~/.local/bin/  # or wherever you installed it
```

## System Requirements

### Minimum Requirements

- **OS:** Linux
- **Kernel:** Any (graceful fallback if no Landlock)
- **Rust:** 1.70+ (for building)
- **Terminal:** Real TTY required

### Recommended for Full Isolation

- **OS:** Linux
- **Kernel:** 5.13 or later (for Landlock support)
- **LSM:** Landlock enabled in kernel config

### Checking Your System

```bash
# Check kernel version
uname -r

# Check if Landlock is available
cat /sys/kernel/security/lsm | grep landlock

# If you see "landlock" in the output, you're good!
```

## Development vs Release Builds

### Debug Build (for development)
```bash
cargo build
# Binary at: target/debug/dshell
# Slower, includes debug symbols
```

### Release Build (for production use)
```bash
cargo build --release
# Binary at: target/release/dshell
# Optimized, faster, smaller
```

**Always use release builds for actual use!**

## Integration Examples

### With Git

```bash
# Work on a repository safely
cd ~/repos/my-project
dshell
dshell> claude
# Claude can only access this git repository
```

### With Docker

```bash
# Run dshell inside a container for even more isolation
docker run -it --rm \
  -v "$(pwd):/workspace" \
  -v "$HOME/.claude:/root/.claude" \
  -w /workspace \
  rust:latest \
  /path/to/dshell
```

### With tmux/screen

```bash
# Create a dedicated session
tmux new -s dshell-session
cd ~/project
dshell
# Detach with Ctrl+B then D
# Reattach with: tmux attach -t dshell-session
```

## Getting Help

- **Documentation:** See [USAGE.md](USAGE.md)
- **Configuration:** See [CONFIGURING_PATHS.md](CONFIGURING_PATHS.md)
- **Technical details:** See [FILESYSTEM_ISOLATION_PLAN.md](FILESYSTEM_ISOLATION_PLAN.md)
- **Built-in help:** Run `dshell> help` inside dshell

## Summary

| Step | Command |
|------|---------|
| **Build** | `cargo build --release` |
| **Install (user)** | `cp target/release/dshell ~/.local/bin/` |
| **Install (system)** | `sudo cp target/release/dshell /usr/local/bin/` |
| **Run** | `cd <project> && dshell` |
| **Configure** | Edit `src/config.rs`, rebuild |
| **Update** | Rebuild and copy new binary |
| **Uninstall** | `rm ~/.local/bin/dshell` |

**Quick start:**
```bash
cargo build --release
alias dshell='/work/oor/shell/target/release/dshell'
cd ~/my-project
dshell
```

That's it! 🎉
