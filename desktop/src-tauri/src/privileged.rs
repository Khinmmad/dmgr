//! Runs root-only operations through the existing `dmgr-polkit-helper` via `pkexec`.
//! Falls back to a direct `dmgr-core` call when the process is already root.

use std::path::PathBuf;
use std::process::Command;

/// Locate the privileged helper binary. Checks PATH (installed) then the repo
/// build outputs so it works both installed and from a dev checkout.
fn helper_path() -> Option<PathBuf> {
    // Installed location / anything on PATH.
    if let Ok(out) = Command::new("sh")
        .arg("-c")
        .arg("command -v dmgr-polkit-helper")
        .output()
    {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    // Dev checkout: target/{release,debug}/dmgr-polkit-helper relative to this crate.
    let manifest = env!("CARGO_MANIFEST_DIR"); // .../desktop/src-tauri
    let root = PathBuf::from(manifest)
        .parent() // desktop
        .and_then(|p| p.parent()) // repo root
        .map(|p| p.to_path_buf());
    if let Some(root) = root {
        for profile in ["release", "debug"] {
            let cand = root.join("target").join(profile).join("dmgr-polkit-helper");
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_root() -> bool {
    extern "C" {
        #[link_name = "geteuid"]
        fn geteuid() -> u32;
    }
    // SAFETY: geteuid is always safe.
    unsafe { geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_root() -> bool {
    // No euid concept on Windows; elevation is handled by the OS/UAC.
    false
}

fn has_pkexec() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("command -v pkexec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether privileged operations can be performed at all (already root, or a
/// usable helper + pkexec are present). Surfaced to the UI so it can warn early.
pub fn can_elevate() -> bool {
    is_root() || (helper_path().is_some() && has_pkexec())
}

/// Run a helper subcommand. `args` are the helper arguments (e.g. `["unbind", path]`).
pub fn run_privileged(args: &[&str]) -> Result<(), String> {
    // Already root: call the core directly, no pkexec prompt needed.
    if is_root() {
        return run_direct(args);
    }

    let helper = helper_path().ok_or_else(|| {
        "Privileged helper 'dmgr-polkit-helper' not found. Install dmgr (or `cargo build` it), \
         then retry. Alternatively, launch dmgr as root."
            .to_string()
    })?;

    if !has_pkexec() {
        return Err(
            "'pkexec' (polkit) not found, so this action can't request root from the GUI. \
             Install polkit, or run dmgr from a terminal as root."
                .to_string(),
        );
    }

    let status = Command::new("pkexec")
        .arg(helper)
        .args(args)
        .status()
        .map_err(|e| format!("pkexec failed to launch: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        match status.code() {
            Some(126) => Err("Authorization dialog dismissed".to_string()),
            Some(127) => Err("Not authorized (polkit denied the action)".to_string()),
            Some(c) => Err(format!("Privileged operation failed (exit {c})")),
            None => Err("Privileged operation terminated by signal".to_string()),
        }
    }
}

/// Resolve a program to an absolute path (pkexec requires a full path).
fn which(prog: &str) -> Option<String> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {prog}"))
        .output()
        .ok()?;
    if out.status.success() {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!p.is_empty()).then_some(p)
    } else {
        None
    }
}

/// Run an arbitrary system program with root via pkexec (e.g. modprobe).
/// Used for actions the dedicated helper doesn't cover.
pub fn run_pkexec(prog: &str, args: &[&str]) -> Result<(), String> {
    let full = which(prog).ok_or_else(|| format!("'{prog}' not found"))?;

    if is_root() {
        let status = Command::new(&full)
            .args(args)
            .status()
            .map_err(|e| format!("{prog} failed: {e}"))?;
        return if status.success() {
            Ok(())
        } else {
            Err(format!("{prog} {} failed", args.join(" ")))
        };
    }

    if !has_pkexec() {
        return Err("'pkexec' (polkit) not found — run dmgr as root for this action".into());
    }

    let status = Command::new("pkexec")
        .arg(&full)
        .args(args)
        .status()
        .map_err(|e| format!("pkexec failed to launch: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        match status.code() {
            Some(126) => Err("Authorization dialog dismissed".into()),
            Some(127) => Err("Not authorized (polkit denied the action)".into()),
            Some(c) => Err(format!("{prog} failed (exit {c})")),
            None => Err(format!("{prog} terminated by signal")),
        }
    }
}

/// Direct core call when running as root.
fn run_direct(args: &[&str]) -> Result<(), String> {
    use dmgr_core::{control, properties};
    let map = |r: dmgr_core::error::Result<()>| r.map_err(|e| e.to_string());
    match args {
        ["bind", path, driver] => map(control::bind_driver(path, driver)),
        ["unbind", path] => map(control::unbind_driver(path)),
        ["set", path, prop, val] => map(properties::set_property(path, prop, val)),
        _ => Err(format!("Unknown privileged command: {args:?}")),
    }
}
