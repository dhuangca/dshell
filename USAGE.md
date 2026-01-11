# Using dshell with Filesystem Isolation

## Quick Start

### Step 1: Navigate to Your Working Directory

```bash
cd /path/to/your/project
```

### Step 2: Run dshell

```bash
/work/oor/shell/target/release/dshell
```

The current directory (`/path/to/your/project`) is now the **root** that isolated commands can access.

### Step 3: Check the Status

When dshell starts, you'll see:

```
Welcome to dshell terminal!

🔒 Security Features:

  • Environment Variables: Filtered by default
    Allowed: HOME, PATH, USER, SHELL, TERM, LANG, EDITOR, COLORTERM
    Use 'allow <VAR>' or 'deny <VAR>' to modify permissions

  • Filesystem Isolation: ENABLED (Landlock)
    Landlock ABI version: V2
    Interactive commands restricted to current directory
    ✓ Kernel-enforced - cannot be bypassed
```

**Key line:** "Interactive commands restricted to current directory"

This means: Any **interactive command** you run can ONLY access files inside your current directory.

## Which Commands Are Isolated?

### Commands that ARE isolated (restricted to current directory):

These commands defined in `src/config.rs::INTERACTIVE_COMMANDS`:
- `claude` - Claude Code ⭐ (main use case)
- `vim`, `nvim`, `nano`, `emacs` - Text editors
- `less`, `more` - File viewers
- `python`, `node`, `irb` - Interactive interpreters
- `bash`, `sh` - Shells
- `ssh` - SSH client
- `top`, `htop` - System monitors
- `man` - Manual viewer
- `ollama` - Ollama AI

When you run these, you'll see:
```
🔒 Filesystem isolated to: /path/to/your/project
```

### Commands that are NOT isolated:

Everything else runs normally without restrictions:
- `cat`, `ls`, `grep`, `find`, `cp`, `mv` - File operations
- Regular commands that don't need isolation

**Why?** These commands:
- Run quickly and exit
- Don't have terminal interaction needs
- Are generally safe utilities

## Example Usage

### Example 1: Running Claude Code Safely

```bash
# Navigate to your project
cd ~/my-project

# Start dshell
/work/oor/shell/target/release/dshell

# In dshell, run Claude
dshell> claude
```

**Result:**
```
🔒 Filesystem isolated to: /home/user/my-project

# Claude Code starts
# Claude can ONLY access files in ~/my-project
# Claude CANNOT access ~/my-project/../other-project
# Claude CANNOT access /etc/passwd
# Claude CANNOT access /tmp/anything
```

### Example 2: Running vim with Isolation

```bash
cd ~/documents/writing
/work/oor/shell/target/release/dshell

dshell> vim essay.txt
```

**What vim can do:**
- ✅ Open/edit `essay.txt` (in current directory)
- ✅ Open/edit `drafts/outline.txt` (in subdirectory)
- ✅ Create new files in current directory
- ✅ Read/write any file under `~/documents/writing/`

**What vim CANNOT do:**
- ❌ Open `../passwords.txt` (parent directory)
- ❌ Open `/etc/passwd` (absolute path)
- ❌ Write to `/tmp/` (outside current directory)
- ❌ Access `~/documents/secrets/` (different directory)

### Example 3: Running Python Interpreter Safely

```bash
cd ~/python-projects/sandbox
/work/oor/shell/target/release/dshell

dshell> python
```

In Python:
```python
>>> # This works - current directory
>>> open('test.txt', 'w').write('hello')

>>> # This FAILS - parent directory
>>> open('../secrets.txt', 'r').read()
PermissionError: [Errno 13] Permission denied: '../secrets.txt'

>>> # This FAILS - absolute path
>>> open('/etc/passwd', 'r').read()
PermissionError: [Errno 13] Permission denied: '/etc/passwd'
```

### Example 4: Running bash Script with Isolation

```bash
cd ~/safe-workspace
/work/oor/shell/target/release/dshell

dshell> bash
```

Inside the bash shell:
```bash
# Works
$ cat file.txt
$ ls -la
$ mkdir subdir
$ cd subdir

# All FAIL with "Permission denied"
$ cat ../../sensitive.txt
$ cat /etc/hostname
$ touch /tmp/test.txt
```

## Understanding the Isolation

### What Happens When You Run an Interactive Command

```
┌─────────────────────────────────────────┐
│ You're in: /home/user/project          │
│ You run: dshell> claude                │
└─────────────┬───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ dshell applies Landlock restriction:   │
│ - Allowed root: /home/user/project     │
│ - Everything else: BLOCKED             │
└─────────────┬───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ Claude Code runs                        │
│                                         │
│ Filesystem view:                        │
│   / (appears to be /home/user/project)  │
│   ├── file1.txt      ✅ accessible     │
│   ├── subdir/        ✅ accessible     │
│   └── README.md      ✅ accessible     │
│                                         │
│ Outside this directory:                 │
│   /home/user/other-project  ❌ blocked  │
│   /etc/                     ❌ blocked  │
│   /tmp/                     ❌ blocked  │
└─────────────────────────────────────────┘
```

### The Current Directory Becomes the Root

Whatever directory you're in when you **start dshell** becomes the "root" for isolated commands.

**Example:**

```bash
# Scenario 1: Isolate to entire home directory
cd ~
dshell
dshell> claude
# Claude can access anything in ~/
# Claude CANNOT access /etc/, /tmp/, etc.

# Scenario 2: Isolate to specific project
cd ~/projects/my-app
dshell
dshell> claude
# Claude can ONLY access ~/projects/my-app/
# Claude CANNOT access ~/projects/other-app/
# Claude CANNOT access ~/documents/
```

## Checking If Isolation Is Active

### Method 1: Check Startup Message

Look for:
```
• Filesystem Isolation: ENABLED (Landlock)
```

If you see:
```
• Filesystem Isolation: NOT AVAILABLE
```

Landlock is not available (older kernel, not enabled, etc.)

### Method 2: Check Environment Variable

When running an isolated command, check:
```bash
dshell> bash
$ echo $DSHELL_ISOLATION_STATUS
fully_enforced    # ✅ Isolation is working
```

Possible values:
- `fully_enforced` - ✅ Full isolation active
- `partially_enforced` - ⚠️ Partial isolation (some features unavailable)
- `not_enforced` - ❌ Isolation failed to apply
- `not_available` - ❌ Landlock not available on system

### Method 3: Test File Access

```bash
cd /tmp/test
echo "secret" > ../secret.txt
echo "allowed" > allowed.txt

dshell
dshell> bash
$ cat allowed.txt       # Should work ✅
$ cat ../secret.txt     # Should fail ❌ Permission denied
```

## Setting Up for Daily Use

### Option 1: Add Alias

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
alias dshell='/work/oor/shell/target/release/dshell'
```

Usage:
```bash
cd ~/my-project
dshell
```

### Option 2: Install Globally

```bash
sudo cp /work/oor/shell/target/release/dshell /usr/local/bin/
```

Usage:
```bash
cd ~/my-project
dshell
```

### Option 3: Wrapper Script for Specific Use Cases

Create `~/bin/claude-safe`:

```bash
#!/bin/bash
# Run Claude Code in isolated environment

if [ $# -eq 0 ]; then
    PROJECT_DIR="."
else
    PROJECT_DIR="$1"
fi

cd "$PROJECT_DIR" || exit 1
echo "Isolating Claude to: $(pwd)"
/work/oor/shell/target/release/dshell
```

Usage:
```bash
claude-safe ~/my-project
# Starts dshell in ~/my-project with isolation
```

## Common Workflows

### Workflow 1: Safe AI Coding Assistant

```bash
# 1. Navigate to project
cd ~/projects/web-app

# 2. Start isolated shell
dshell

# 3. Run AI assistant (isolated to project)
dshell> claude

# Claude can now help with your project
# but cannot access files outside ~/projects/web-app
```

### Workflow 2: Testing Untrusted Scripts

```bash
# 1. Create isolated test directory
mkdir -p ~/sandbox/test
cd ~/sandbox/test

# 2. Copy script to test
cp ~/downloads/untrusted-script.sh .

# 3. Run in isolation
dshell
dshell> bash untrusted-script.sh

# Script can only affect files in ~/sandbox/test
# Your important files are safe
```

### Workflow 3: Collaborative Editing

```bash
# 1. Go to shared project
cd ~/shared/team-project

# 2. Run isolated editor
dshell
dshell> vim shared-doc.txt

# You can edit team files
# But cannot accidentally access your personal ~/documents/
```

## Advanced Configuration

### Add More Commands to Isolation List

Edit `src/config.rs`:

```rust
pub const INTERACTIVE_COMMANDS: &[&str] = &[
    "claude",
    "vim",
    "nvim",
    // Add your custom commands here:
    "mycustomtool",
    "anothertool",
];
```

Then rebuild:
```bash
cargo build --release
```

### Remove Commands from Isolation

If you want a command to run WITHOUT isolation, remove it from the `INTERACTIVE_COMMANDS` list.

## Troubleshooting

### "Filesystem Isolation: NOT AVAILABLE"

**Cause:** Landlock not available on your system

**Solutions:**
1. Check kernel version: `uname -r` (need 5.13+)
2. Check LSMs loaded: `cat /sys/kernel/security/lsm | grep landlock`
3. If kernel is too old: Upgrade kernel or use Docker
4. If Landlock not loaded: Enable in kernel config

### Commands Still Access Parent Directory

**Possible causes:**

1. **Command is not in INTERACTIVE_COMMANDS list**
   - Check `src/config.rs`
   - Regular commands like `cat`, `ls` are intentionally not isolated

2. **Isolation status shows "not_enforced" or "not_available"**
   - Check `echo $DSHELL_ISOLATION_STATUS` inside command
   - If not "fully_enforced", isolation isn't working

3. **Testing wrong command**
   - Test with: `bash`, `python`, `vim` (known interactive commands)
   - Don't test with: `cat`, `ls`, `grep` (not isolated)

### Error: "No such device or address"

**Cause:** Running dshell without a proper terminal (TTY)

**Wrong:**
```bash
echo "ls" | dshell          # ❌ Piped input
dshell < /dev/null          # ❌ Redirected stdin
./script.sh                 # ❌ Script that runs dshell
```

**Right:**
```bash
dshell                      # ✅ Interactive terminal
```

## Summary

| Aspect | Details |
|--------|---------|
| **Current directory** | Becomes the isolated root |
| **Isolated commands** | claude, vim, bash, python, etc. (see INTERACTIVE_COMMANDS) |
| **Not isolated** | cat, ls, grep, etc. (regular utilities) |
| **Isolation method** | Landlock LSM (kernel-enforced) |
| **Can be bypassed?** | No (when fully_enforced) |
| **Requirements** | Linux 5.13+, TTY |
| **Use case** | Running AI assistants, untrusted tools, sandboxing |

**Bottom line:** Just `cd` to your project and run `dshell`. Interactive commands will be locked to that directory. ✅
