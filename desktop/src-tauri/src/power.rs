//! Power management: power-profiles-daemon profiles, battery, backlight.
//! Linux only; everything degrades gracefully when a piece isn't present.

use serde::Serialize;

#[derive(Serialize, Default)]
pub struct PowerInfo {
    pub has_ppd: bool, // power-profiles-daemon present
    pub profiles: Vec<String>,
    pub active_profile: String,
    pub battery_percent: Option<u8>,
    pub battery_status: String, // Charging | Discharging | Full | ""
    pub has_brightness: bool,
    pub brightness_percent: Option<u8>,
}

#[cfg(target_os = "linux")]
pub fn info() -> PowerInfo {
    use std::process::Command;
    let mut p = PowerInfo::default();

    if let Ok(out) = Command::new("powerprofilesctl").arg("list").output() {
        if out.status.success() {
            let (profiles, active) = parse_profiles(&String::from_utf8_lossy(&out.stdout));
            if !profiles.is_empty() {
                p.has_ppd = true;
                p.profiles = profiles;
                p.active_profile = active;
            }
        }
    }

    if let Some((pct, status)) = read_battery() {
        p.battery_percent = Some(pct);
        p.battery_status = status;
    }

    if let Some(pct) = read_brightness() {
        p.has_brightness = true;
        p.brightness_percent = Some(pct);
    }

    p
}

#[cfg(not(target_os = "linux"))]
pub fn info() -> PowerInfo {
    PowerInfo::default()
}

#[cfg(target_os = "linux")]
pub fn set_profile(profile: &str) -> Result<(), String> {
    if !matches!(profile, "power-saver" | "balanced" | "performance") {
        return Err("unknown power profile".into());
    }
    let st = std::process::Command::new("powerprofilesctl")
        .args(["set", profile])
        .status()
        .map_err(|e| format!("powerprofilesctl not found: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err("failed to set power profile".into())
    }
}

#[cfg(target_os = "linux")]
pub fn set_brightness(percent: u8) -> Result<(), String> {
    // brightnessctl ships a udev rule, so this works without root.
    let st = std::process::Command::new("brightnessctl")
        .args(["set", &format!("{}%", percent.min(100))])
        .status()
        .map_err(|_| "brightnessctl not found — install it to control brightness".to_string())?;
    if st.success() {
        Ok(())
    } else {
        Err("failed to set brightness".into())
    }
}

#[cfg(not(target_os = "linux"))]
pub fn set_profile(_profile: &str) -> Result<(), String> {
    Err("not supported on this platform".into())
}

#[cfg(not(target_os = "linux"))]
pub fn set_brightness(_percent: u8) -> Result<(), String> {
    Err("not supported on this platform".into())
}

#[cfg(target_os = "linux")]
fn read_battery() -> Option<(u8, String)> {
    for entry in std::fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let path = entry.path();
        if std::fs::read_to_string(path.join("type")).unwrap_or_default().trim() == "Battery" {
            let pct = std::fs::read_to_string(path.join("capacity"))
                .ok()?
                .trim()
                .parse::<u8>()
                .ok()?;
            let status = std::fs::read_to_string(path.join("status"))
                .unwrap_or_default()
                .trim()
                .to_string();
            return Some((pct, status));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_brightness() -> Option<u8> {
    for entry in std::fs::read_dir("/sys/class/backlight").ok()?.flatten() {
        let path = entry.path();
        let cur = std::fs::read_to_string(path.join("brightness"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        let max = std::fs::read_to_string(path.join("max_brightness"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        if let (Some(c), Some(m)) = (cur, max) {
            if m > 0 {
                return Some(((c * 100 / m) as u8).min(100));
            }
        }
    }
    None
}

/// Parse `powerprofilesctl list`. Active profile is prefixed with `*`.
/// Only the three power-profiles-daemon profiles are recognised, which keeps
/// indented `Property:` lines from being mistaken for profile headers.
#[cfg_attr(windows, allow(dead_code))]
fn parse_profiles(text: &str) -> (Vec<String>, String) {
    let mut profiles = Vec::new();
    let mut active = String::new();
    for line in text.lines() {
        let t = line.trim();
        let (is_active, body) = match t.strip_prefix('*') {
            Some(rest) => (true, rest.trim()),
            None => (false, t),
        };
        if let Some(name) = body.strip_suffix(':') {
            if matches!(name, "power-saver" | "balanced" | "performance") {
                profiles.push(name.to_string());
                if is_active {
                    active = name.to_string();
                }
            }
        }
    }
    (profiles, active)
}

#[cfg(test)]
mod tests {
    use super::parse_profiles;

    #[test]
    fn parses_ppd_list_with_active() {
        let sample = "\
  performance:
    Driver:     platform_profile

* balanced:
    Driver:     platform_profile

  power-saver:
    Driver:     platform_profile
";
        let (profiles, active) = parse_profiles(sample);
        assert_eq!(profiles, vec!["performance", "balanced", "power-saver"]);
        assert_eq!(active, "balanced");
    }
}
