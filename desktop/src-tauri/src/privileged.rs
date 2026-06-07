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

fn is_root() -> bool {
    // SAFETY: getuid is always safe.
    unsafe { libc_geteuid() == 0 }
}

extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

/// Run a helper subcommand. `args` are the helper arguments (e.g. `["unbind", path]`).
pub fn run_privileged(args: &[&str]) -> Result<(), String> {
    // Already root: call the core directly, no pkexec prompt needed.
    if is_root() {
        return run_direct(args);
    }

    let helper = helper_path()
        .ok_or_else(|| "dmgr-polkit-helper not found (install dmgr or run `cargo build`)".to_string())?;

    let status = Command::new("pkexec")
        .arg(helper)
        .args(args)
        .status()
        .map_err(|e| format!("pkexec failed to launch: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        match status.code() {
            Some(126) | Some(127) => Err("Authorization cancelled or denied".to_string()),
            Some(c) => Err(format!("Privileged operation failed (exit {c})")),
            None => Err("Privileged operation terminated by signal".to_string()),
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
