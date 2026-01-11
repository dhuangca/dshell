# Configuring Additional Allowed Paths

## Overview

By default, dshell with Landlock isolation allows interactive commands to access:

**✅ Always Accessible:**
- Current working directory (read/write/execute)
- `/usr`, `/bin`, `/lib`, `/lib64` (read-only + execute)
- `/etc` (read-only)
- `/dev`, `/proc`, `/sys` (read-only)
- `/tmp`, `/var/tmp` (read/write for temporary files)

**❌ Blocked by default:**
- Parent directories (`../`)
- Absolute paths outside allowed directories
- User home directory (except configured paths)

## Why Add Additional Paths?

Some applications need access to specific configuration files or data directories in your home directory:

- **Claude Code** needs `~/.claude/` and `~/.claude.json`
- **Node.js tools** might need `~/.npm/` or `~/.nvm/`
- **Other tools** might need `~/.config/app/` or `~/.local/share/app/`

## How to Add Paths

### Step 1: Edit the Configuration

Open `src/config.rs` in your editor:

```bash
vim src/config.rs
```

### Step 2: Find the `ADDITIONAL_ALLOWED_PATHS` Section

Look for this section (around line 36):

```rust
/// Additional paths to allow access to for isolated commands
/// These paths will have read-write access
/// Users can add paths here for tools like Claude that need access to config files
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    // Claude Code configuration and data
    "~/.claude",           // Claude config directory
    "~/.claude.json",      // Claude config file

    // Add more paths here as needed, for example:
    // "~/.config/myapp",
    // "~/.local/share/myapp",
];
```

### Step 3: Add Your Paths

Add your paths to the array. You can use:

**Tilde notation** for home directory:
```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    "~/.claude",
    "~/.claude.json",
    "~/.config/myapp",        // Add this
    "~/.local/share/myapp",   // Add this
    "~/my-important-files",   // Add this
];
```

**Note:**
- ✅ Use `~/.path` for paths in home directory
- ✅ Use `/absolute/path` for absolute paths
- ❌ Do NOT use relative paths like `../path`

### Step 4: Rebuild dshell

After editing the configuration:

```bash
cargo build --release
```

### Step 5: Test

Run dshell and verify your paths are accessible:

```bash
cd /tmp/test
/work/oor/shell/target/release/dshell

dshell> bash
$ cat ~/.claude.json  # Should work now
$ ls ~/.claude/       # Should work now
```

## Examples

### Example 1: Node.js and npm

If you're running Node.js tools that need npm/nvm:

```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    "~/.claude",
    "~/.claude.json",
    "~/.npm",              // npm cache
    "~/.nvm",              // nvm (Node Version Manager)
    "~/.node_repl_history", // Node REPL history
];
```

### Example 2: Python Development

If you're running Python tools:

```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    "~/.claude",
    "~/.claude.json",
    "~/.python_history",   // Python REPL history
    "~/.pyenv",            // pyenv
    "~/.local/lib/python", // Python packages
];
```

### Example 3: Git Configuration

If you need git config:

```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    "~/.claude",
    "~/.claude.json",
    "~/.gitconfig",        // Git global config
    "~/.ssh",              // SSH keys (be careful!)
];
```

### Example 4: Multiple Applications

```rust
pub const ADDITIONAL_ALLOWED_PATHS: &[&str] = &[
    // Claude
    "~/.claude",
    "~/.claude.json",

    // VS Code
    "~/.vscode",

    // Custom configs
    "~/.config/myapp",
    "~/.local/share/myapp",

    // Development tools
    "~/.cargo",            // Rust cargo
    "~/.rustup",           // Rust toolchain
];
```

## Security Considerations

### ⚠️ Think Before Adding Paths

Each path you add reduces isolation. Consider:

**Low Risk:**
- ✅ Application-specific config: `~/.claude/`, `~/.myapp/`
- ✅ Cache directories: `~/.cache/myapp/`
- ✅ Read-only data: `~/.local/share/myapp/`

**Medium Risk:**
- ⚠️ Version control: `~/.gitconfig`
- ⚠️ Development tools: `~/.cargo/`, `~/.rustup/`

**High Risk:**
- ❌ SSH keys: `~/.ssh/` (gives access to your credentials!)
- ❌ Entire home: `~` (defeats the purpose of isolation!)
- ❌ System paths: `/root/`, `/var/` (dangerous)

### Best Practices

1. **Be specific:** Use `~/.claude/` instead of `~`
2. **Minimal access:** Only add what's actually needed
3. **Test first:** Run the command and see what it complains about
4. **Document why:** Add comments explaining why each path is needed
5. **Review regularly:** Remove paths that are no longer needed

## How It Works

### Path Expansion

Paths starting with `~` are automatically expanded:
- `~/.claude` → `/home/username/.claude`
- `~/documents` → `/home/username/documents`

### Access Level

All paths in `ADDITIONAL_ALLOWED_PATHS` get **full read-write access**:
- ✅ Read files
- ✅ Write files
- ✅ Create files
- ✅ Delete files
- ✅ Execute binaries

### Path Types

**Directories:**
- Adding `~/.claude` allows access to everything inside `~/.claude/`
- Recursive: includes all subdirectories

**Files:**
- Adding `~/.claude.json` allows access to just that file
- Does NOT include the parent directory

## Troubleshooting

### Path Not Working?

**1. Check if path exists:**
```bash
ls -la ~/.claude
```

If the path doesn't exist when dshell starts, it won't be added to the allowed list.

**2. Check expansion:**
The path must be either:
- Starts with `~/` (tilde + slash)
- Equals `~` (just tilde)
- Absolute path `/path/to/file`

**3. Rebuild:**
Did you rebuild after editing?
```bash
cargo build --release
```

**4. Test without dshell:**
Make sure the application works normally:
```bash
# Outside dshell
claude  # Does this work?
```

### Claude Still Can't Access Files?

If Claude still can't access its config:

**Check what Claude is trying to access:**
```bash
strace -e open,openat claude 2>&1 | grep -i claude
```

This shows all files Claude tries to open. Add the missing paths to the config.

### Error: "Permission denied"

If you get permission denied for a path you added:

1. **Check ownership:** `ls -la ~/.claude`
2. **Check permissions:** Make sure you can read/write the path
3. **Check spelling:** Typos in config.rs?
4. **Check rebuild:** Did you rebuild after editing?

## Advanced: Dynamic Path Detection

If you want paths to be detected automatically (advanced users):

You could modify `src/security/landlock.rs` to automatically detect tool-specific paths:

```rust
// Example: Automatically add .claude if it exists
let potential_paths = vec![
    "~/.claude",
    "~/.config/claude",
    // etc
];

for path in potential_paths {
    let expanded = Self::expand_tilde(path);
    if Path::new(&expanded).exists() {
        // Add to ruleset
    }
}
```

But this is not recommended - explicit configuration is clearer and more predictable.

## Summary

| Aspect | Details |
|--------|---------|
| **Where to configure** | `src/config.rs` → `ADDITIONAL_ALLOWED_PATHS` |
| **Path format** | `~/.path` for home, `/absolute/path` for others |
| **Access level** | Full read-write access |
| **Takes effect** | After `cargo build --release` |
| **Already included** | `~/.claude`, `~/.claude.json` |
| **Be careful with** | SSH keys, credentials, entire home directory |

**Remember:** Only add paths that are truly necessary. Each additional path reduces the security isolation!
