# File System Isolation Implementation Plan

## Overview

This document outlines multiple approaches to implement true file system isolation for dshell, focusing on Linux-specific solutions that can actually restrict file access.

---

## Option 1: Mount Namespace + Bind Mount + pivot_root (Recommended)

### Concept
Use Linux mount namespaces to create an isolated filesystem view where the interactive command can only see the current directory as its root.

### Requirements
- Linux kernel 3.8+ (for unprivileged user namespaces)
- `/proc/sys/kernel/unprivileged_userns_clone` enabled (set to 1)
- Rust dependencies: `nix` or `libc` crate for syscalls

### Architecture

```
┌─────────────────────────────────────────┐
│ dshell (parent process)                 │
│ - Runs with normal permissions          │
│ - Creates new mount namespace           │
└──────────────┬──────────────────────────┘
               │ fork + unshare(CLONE_NEWNS | CLONE_NEWUSER)
               ▼
┌─────────────────────────────────────────┐
│ Isolated child process                  │
│ - Has its own mount namespace           │
│ - pivot_root to working directory       │
│ - Old root is unmounted                 │
│ - Can only see files in working dir     │
└─────────────────────────────────────────┘
```

### Implementation Steps

#### Step 1: Add Dependencies
```toml
# Cargo.toml
[dependencies]
nix = { version = "0.27", features = ["user", "mount", "sched"] }
libc = "0.2"
```

#### Step 2: Create Isolation Module
```rust
// src/security/fs_isolation.rs

use nix::sched::{unshare, CloneFlags};
use nix::mount::{mount, umount2, MsFlags, MntFlags};
use nix::unistd::{pivot_root, chdir};
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

pub struct FilesystemIsolation {
    work_dir: PathBuf,
}

impl FilesystemIsolation {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Create isolated filesystem using mount namespace
    pub fn isolate(&self) -> io::Result<()> {
        // Step 1: Create new mount and user namespaces
        unshare(CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWUSER)
            .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;

        // Step 2: Set up UID/GID mappings for unprivileged namespace
        self.setup_uid_gid_map()?;

        // Step 3: Create temporary directory for new root
        let new_root = self.work_dir.join(".dshell_root");
        fs::create_dir_all(&new_root)?;

        // Step 4: Bind mount working directory to new root
        mount(
            Some(&self.work_dir),
            &new_root,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Step 5: Create put_old directory for old root
        let put_old = new_root.join(".old_root");
        fs::create_dir_all(&put_old)?;

        // Step 6: Change to new root directory
        chdir(&new_root)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Step 7: Pivot root
        pivot_root(".", &put_old)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Step 8: Change to new root
        chdir("/")
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Step 9: Unmount old root
        umount2("/.old_root", MntFlags::MNT_DETACH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Step 10: Remove old root directory
        fs::remove_dir("/.old_root")?;

        Ok(())
    }

    fn setup_uid_gid_map(&self) -> io::Result<()> {
        let pid = std::process::id();
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        // Write UID mapping
        fs::write(
            format!("/proc/{}/uid_map", pid),
            format!("0 {} 1\n", uid),
        )?;

        // Deny setgroups
        fs::write(
            format!("/proc/{}/setgroups", pid),
            "deny\n",
        )?;

        // Write GID mapping
        fs::write(
            format!("/proc/{}/gid_map", pid),
            format!("0 {} 1\n", gid),
        )?;

        Ok(())
    }
}
```

#### Step 3: Modify Executor
```rust
// src/shell/executor.rs

use crate::security::fs_isolation::FilesystemIsolation;
use std::env;
use std::process::{Command, Stdio};

pub fn execute_interactive_isolated(
    cmd: &ParsedCommand,
    permissions: &PermissionManager,
    custom_env: &HashMap<String, String>,
) -> io::Result<i32> {
    let work_dir = env::current_dir()?;

    // Fork process
    match unsafe { libc::fork() } {
        -1 => Err(io::Error::last_os_error()),
        0 => {
            // Child process
            // Create filesystem isolation
            let isolation = FilesystemIsolation::new(work_dir);
            if let Err(e) = isolation.isolate() {
                eprintln!("Failed to isolate filesystem: {}", e);
                std::process::exit(1);
            }

            // Now execute command in isolated environment
            let mut command = Command::new(&cmd.command);
            command.args(&cmd.args);
            command.env_clear();

            // Add environment variables
            for (key, value) in custom_env {
                command.env(key, value);
            }
            for (key, value) in permissions.get_allowed_env_vars() {
                if !value.starts_with("[REDACTED") && !custom_env.contains_key(&key) {
                    command.env(key, value);
                }
            }

            // Execute
            let status = command
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .unwrap();

            std::process::exit(status.code().unwrap_or(-1));
        }
        child_pid => {
            // Parent process - wait for child
            let mut status: i32 = 0;
            unsafe {
                libc::waitpid(child_pid, &mut status, 0);
            }
            Ok(libc::WEXITSTATUS(status))
        }
    }
}
```

#### Step 4: Configuration
```rust
// src/config.rs

/// Enable filesystem isolation for these commands
pub const FS_ISOLATION_ENABLED: bool = true;

/// Commands that will run in isolated filesystem
pub const FS_ISOLATED_COMMANDS: &[&str] = &[
    "claude",
    "vim",
    "nvim",
    "nano",
    "emacs",
];
```

### Pros
- ✅ True filesystem isolation
- ✅ No root privileges required (user namespaces)
- ✅ Works on most modern Linux systems
- ✅ Relatively simple implementation

### Cons
- ❌ Linux-only (won't work on macOS, Windows)
- ❌ Requires unprivileged user namespaces enabled
- ❌ Some Linux distributions disable unprivileged namespaces by default
- ❌ Child process sees different filesystem view (might confuse some tools)

---

## Option 2: Seccomp-BPF + Path Validation

### Concept
Use seccomp (Secure Computing Mode) to intercept file-related syscalls and validate paths before allowing them.

### Architecture

```
┌─────────────────────────────────────────┐
│ dshell                                  │
└──────────────┬──────────────────────────┘
               │ Install seccomp filter
               ▼
┌─────────────────────────────────────────┐
│ Seccomp Filter (BPF program)            │
│ - Intercepts: open, openat, creat, etc. │
│ - Validates path arguments              │
│ - Allows: paths within work_dir         │
│ - Denies: ../, /, absolute paths        │
└─────────────────────────────────────────┘
```

### Implementation Steps

#### Step 1: Add Dependencies
```toml
[dependencies]
seccomp-sys = "0.2"
libseccomp = "0.3"
```

#### Step 2: Create Seccomp Filter
```rust
// src/security/seccomp_filter.rs

use libseccomp::*;
use std::path::Path;

pub struct SeccompFilter {
    allowed_dir: String,
}

impl SeccompFilter {
    pub fn new(allowed_dir: &Path) -> Self {
        Self {
            allowed_dir: allowed_dir.to_string_lossy().to_string(),
        }
    }

    pub fn install(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create filter context
        let mut ctx = ScmpFilterContext::new_filter(ScmpAction::Allow)?;

        // Block file operations with paths outside allowed_dir
        let syscalls = vec![
            ScmpSyscall::from_name("open")?,
            ScmpSyscall::from_name("openat")?,
            ScmpSyscall::from_name("creat")?,
            ScmpSyscall::from_name("unlink")?,
            ScmpSyscall::from_name("unlinkat")?,
            ScmpSyscall::from_name("rename")?,
            ScmpSyscall::from_name("renameat")?,
        ];

        for syscall in syscalls {
            // This is complex - seccomp can't easily validate string arguments
            // This approach has limitations
            ctx.add_rule(ScmpAction::Errno(libc::EPERM as i32), syscall)?;
        }

        ctx.load()?;
        Ok(())
    }
}
```

### Pros
- ✅ Very strong security (kernel-level enforcement)
- ✅ Can't be bypassed by child process

### Cons
- ❌ Extremely complex to implement correctly
- ❌ Seccomp can't easily validate string arguments (paths)
- ❌ Would block ALL file operations, hard to selectively allow
- ❌ Linux-only
- ❌ Can break legitimate operations

**Verdict: Not recommended due to complexity and limitations**

---

## Option 3: chroot (Requires Root)

### Concept
Use traditional chroot to change the root directory for the process.

### Implementation Steps

```rust
// src/security/chroot_isolation.rs

use nix::unistd::chroot;
use std::path::Path;

pub fn isolate_with_chroot(work_dir: &Path) -> io::Result<()> {
    // Requires root privileges
    chroot(work_dir)
        .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;

    chdir("/")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}
```

### Pros
- ✅ Simple and well-understood
- ✅ Strong isolation

### Cons
- ❌ **Requires root privileges**
- ❌ Can be escaped if process runs as root
- ❌ Linux/Unix only

**Verdict: Not suitable for unprivileged shell**

---

## Option 4: Landlock LSM (Linux 5.13+)

### Concept
Use Landlock LSM (Linux Security Module) for path-based access control without root.

### Architecture

```
┌─────────────────────────────────────────┐
│ dshell                                  │
│ - Create landlock ruleset               │
│ - Add allowed path: /work/dir           │
│ - Restrict to: read, write, exec        │
│ - Apply to current thread               │
└──────────────┬──────────────────────────┘
               │ Fork with landlock active
               ▼
┌─────────────────────────────────────────┐
│ Child process (claude, vim, etc.)       │
│ - Can only access /work/dir/*           │
│ - Kernel enforces restrictions          │
│ - Cannot escape isolation               │
└─────────────────────────────────────────┘
```

### Implementation Steps

#### Step 1: Add Dependencies
```toml
[dependencies]
landlock = "0.3"
```

#### Step 2: Create Landlock Isolation
```rust
// src/security/landlock_isolation.rs

use landlock::{
    Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr,
    RulesetStatus, ABI,
};
use std::path::Path;
use std::io;

pub struct LandlockIsolation {
    work_dir: PathBuf,
}

impl LandlockIsolation {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    pub fn restrict(&self) -> io::Result<()> {
        // Create ruleset for filesystem access
        let abi = ABI::V2;

        let access_fs = AccessFs::from_all(abi);

        let status = Ruleset::default()
            .handle_fs(access_fs)?
            .create()?
            .add_rule(
                landlock::PathBeneath::new(
                    &self.work_dir,
                    access_fs,
                )
            )?
            .restrict_self()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        match status.ruleset {
            RulesetStatus::FullyEnforced => {
                println!("✓ Filesystem fully restricted to: {}",
                         self.work_dir.display());
            }
            RulesetStatus::PartiallyEnforced => {
                eprintln!("⚠ Filesystem partially restricted");
            }
            RulesetStatus::NotEnforced => {
                eprintln!("✗ Filesystem restrictions not enforced!");
            }
        }

        Ok(())
    }
}
```

#### Step 3: Integration with Executor
```rust
// src/shell/executor.rs

use crate::security::landlock_isolation::LandlockIsolation;

pub fn execute_interactive_with_landlock(
    cmd: &ParsedCommand,
    permissions: &PermissionManager,
    custom_env: &HashMap<String, String>,
) -> io::Result<i32> {
    let work_dir = env::current_dir()?;

    // Fork before applying landlock
    match unsafe { libc::fork() } {
        -1 => Err(io::Error::last_os_error()),
        0 => {
            // Child process
            // Apply landlock restrictions
            let isolation = LandlockIsolation::new(work_dir);
            if let Err(e) = isolation.restrict() {
                eprintln!("Warning: Could not apply filesystem restrictions: {}", e);
            }

            // Execute command
            let mut command = Command::new(&cmd.command);
            command.args(&cmd.args);
            command.env_clear();

            // Set environment
            for (key, value) in custom_env {
                command.env(key, value);
            }
            for (key, value) in permissions.get_allowed_env_vars() {
                if !value.starts_with("[REDACTED") {
                    command.env(key, value);
                }
            }

            let status = command
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .unwrap();

            std::process::exit(status.code().unwrap_or(-1));
        }
        child_pid => {
            // Parent - wait for child
            let mut status: i32 = 0;
            unsafe { libc::waitpid(child_pid, &mut status, 0) };
            Ok(libc::WEXITSTATUS(status))
        }
    }
}
```

### Pros
- ✅ **No root privileges required**
- ✅ Kernel-enforced (cannot be bypassed)
- ✅ Clean and modern API
- ✅ Designed specifically for this use case
- ✅ Can configure fine-grained permissions (read, write, execute)

### Cons
- ❌ Requires Linux 5.13+ kernel
- ❌ Not available on older systems
- ❌ Linux-only

**Verdict: Best option for modern Linux systems**

---

## Option 5: Docker/Podman Container (External)

### Concept
Wrap dshell execution in a container with volume mounts.

### Implementation

```bash
#!/bin/bash
# dshell-isolated.sh

WORK_DIR=$(pwd)
CONTAINER_IMAGE="rust:latest"

docker run -it --rm \
    --network none \
    -v "$WORK_DIR:/workspace:rw" \
    -v "/path/to/dshell:/usr/local/bin/dshell:ro" \
    -w /workspace \
    --security-opt="no-new-privileges" \
    --cap-drop=ALL \
    "$CONTAINER_IMAGE" \
    /usr/local/bin/dshell
```

### Pros
- ✅ Complete isolation (filesystem, network, process)
- ✅ Works on Linux, macOS, Windows (with Docker Desktop)
- ✅ Well-tested and reliable
- ✅ Can add additional security features easily

### Cons
- ❌ Requires Docker/Podman installed
- ❌ Slower startup time
- ❌ External dependency
- ❌ More complex setup for users

**Verdict: Best option for cross-platform isolation**

---

## Comparison Matrix

| Feature | Mount NS + pivot_root | Seccomp-BPF | chroot | Landlock | Docker |
|---------|---------------------|-------------|--------|----------|--------|
| **No root required** | ✅ | ✅ | ❌ | ✅ | ✅ |
| **Kernel enforcement** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Cannot be bypassed** | ✅ | ✅ | ⚠️ | ✅ | ✅ |
| **Linux only** | ✅ | ✅ | ✅ | ✅ | ❌ |
| **Kernel version** | 3.8+ | 3.5+ | Any | 5.13+ | Any |
| **Implementation complexity** | Medium | Very High | Low | Low | Low |
| **Isolation strength** | Strong | Strong | Medium | Strong | Very Strong |
| **Performance overhead** | Low | Low | Low | Low | Medium |
| **Production ready** | ✅ | ❌ | ✅ | ✅ | ✅ |

---

## Recommended Implementation Strategy

### Phase 1: Landlock (Preferred for Modern Systems)
- Implement Landlock-based isolation
- Falls back to warning-only mode if not available
- Best for Linux 5.13+

### Phase 2: Mount Namespace Fallback
- If Landlock not available, use mount namespace + pivot_root
- Works on Linux 3.8+
- More complex but widely supported

### Phase 3: Warning Mode
- If neither available, show warnings (current implementation)
- Document the need to run in container

### Code Structure

```rust
// src/security/fs_isolation/mod.rs

pub mod landlock;
pub mod mount_namespace;

pub enum IsolationMethod {
    Landlock,
    MountNamespace,
    WarningOnly,
}

pub fn detect_available_method() -> IsolationMethod {
    // Try Landlock first
    if landlock::is_available() {
        return IsolationMethod::Landlock;
    }

    // Try mount namespace
    if mount_namespace::is_available() {
        return IsolationMethod::MountNamespace;
    }

    // Fall back to warnings
    IsolationMethod::WarningOnly
}

pub fn isolate(work_dir: &Path, method: IsolationMethod) -> io::Result<()> {
    match method {
        IsolationMethod::Landlock => landlock::isolate(work_dir),
        IsolationMethod::MountNamespace => mount_namespace::isolate(work_dir),
        IsolationMethod::WarningOnly => {
            eprintln!("⚠️  Warning: Filesystem isolation not available");
            Ok(())
        }
    }
}
```

---

## Next Steps

1. **Choose implementation approach:**
   - Option A: Landlock only (simplest, modern systems)
   - Option B: Landlock + Mount NS fallback (wider compatibility)
   - Option C: Docker wrapper (cross-platform)

2. **Add dependencies to Cargo.toml**

3. **Implement isolation module**

4. **Update executor to use isolation**

5. **Add configuration options**

6. **Test on various systems**

7. **Document requirements and limitations**

Which option would you like me to implement?
