//! Host environment detection: distro, session (X11/Wayland), GPU vendor.
//! Drives the WebKit workaround and distro-aware install hints — so the app
//! adapts instead of assuming Arch/systemd/Nvidia/Wayland.

use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct Platform {
    pub os: String,
    pub distro_id: String,
    pub distro_name: String,
    pub session: String, // wayland | x11 | unknown
    pub gpu_nvidia: bool,
    pub audio_backend: String,
    pub can_elevate: bool,
    pub package_hint: String,
}

pub fn session_type() -> String {
    let xdg = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if std::env::var_os("WAYLAND_DISPLAY").is_some() || xdg == "wayland" {
        "wayland".into()
    } else if std::env::var_os("DISPLAY").is_some() || xdg == "x11" {
        "x11".into()
    } else {
        "unknown".into()
    }
}

pub fn has_nvidia() -> bool {
    Path::new("/sys/module/nvidia").exists()
        || Path::new("/proc/driver/nvidia").exists()
        || std::fs::read_dir("/dev")
            .map(|rd| {
                rd.flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("nvidia"))
            })
            .unwrap_or(false)
}

/// (ID, PRETTY_NAME) from /etc/os-release.
pub fn os_release() -> (String, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let get = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k).map(|v| v.trim().trim_matches('"').to_string()))
    };
    (
        get("ID=").unwrap_or_else(|| "linux".into()),
        get("PRETTY_NAME=").unwrap_or_else(|| "Linux".into()),
    )
}

pub fn package_hint(distro_id: &str) -> String {
    match distro_id {
        "arch" | "endeavouros" | "manjaro" | "cachyos" | "garuda" => {
            "sudo pacman -S <pkg>  (AUR: paru/yay)".into()
        }
        "debian" | "ubuntu" | "pop" | "linuxmint" | "elementary" | "zorin" => {
            "sudo apt install <pkg>".into()
        }
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" | "nobara" => {
            "sudo dnf install <pkg>".into()
        }
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" => "sudo zypper install <pkg>".into(),
        "void" => "sudo xbps-install -S <pkg>".into(),
        "alpine" => "sudo apk add <pkg>".into(),
        "gentoo" => "sudo emerge <pkg>".into(),
        "nixos" => "add <pkg> to your configuration.nix".into(),
        _ => "install <pkg> with your distro's package manager".into(),
    }
}

pub fn detect() -> Platform {
    let (id, name) = os_release();
    let hint = package_hint(&id);
    Platform {
        os: std::env::consts::OS.to_string(),
        distro_id: id,
        distro_name: name,
        session: session_type(),
        gpu_nvidia: has_nvidia(),
        audio_backend: crate::audio::backend_name().to_string(),
        can_elevate: crate::privileged::can_elevate(),
        package_hint: hint,
    }
}

/// Apply rendering workarounds only where they're actually needed.
pub fn apply_webkit_workarounds() {
    #[cfg(target_os = "linux")]
    {
        // Nvidia + Wayland renders a blank WebKitGTK window with the DMABUF path.
        // Leave Intel/AMD (and X11) alone — they benefit from DMABUF.
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none()
            && session_type() == "wayland"
            && has_nvidia()
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::package_hint;

    #[test]
    fn package_hints_per_distro_family() {
        assert!(package_hint("arch").contains("pacman"));
        assert!(package_hint("endeavouros").contains("pacman"));
        assert!(package_hint("ubuntu").contains("apt"));
        assert!(package_hint("fedora").contains("dnf"));
        assert!(package_hint("opensuse-tumbleweed").contains("zypper"));
        assert!(package_hint("void").contains("xbps"));
        assert!(package_hint("nixos").contains("configuration.nix"));
        // unknown distro → generic guidance
        assert!(package_hint("someunknowndistro").contains("package manager"));
    }
}
