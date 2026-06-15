//! systemd service management (Linux): list units and start/stop/restart/etc.
//! Mutating actions go through pkexec.

use serde::Serialize;

#[derive(Serialize)]
pub struct Service {
    pub name: String,
    pub active: String,      // active | inactive | failed
    pub sub: String,         // running | exited | dead | failed
    pub description: String,
}

#[cfg(target_os = "linux")]
pub fn list() -> Vec<Service> {
    let out = std::process::Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--plain",
            "--no-legend",
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => parse_units(&String::from_utf8_lossy(&o.stdout)),
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn list() -> Vec<Service> {
    Vec::new()
}

#[cfg(target_os = "linux")]
pub fn action(name: &str, action: &str) -> Result<(), String> {
    if !valid_unit(name) {
        return Err("invalid service name".into());
    }
    if !matches!(action, "start" | "stop" | "restart" | "enable" | "disable") {
        return Err("invalid action".into());
    }
    crate::privileged::run_pkexec("systemctl", &[action, name])
}

#[cfg(not(target_os = "linux"))]
pub fn action(_name: &str, _action: &str) -> Result<(), String> {
    Err("systemd services aren't available on this platform".into())
}

#[cfg_attr(windows, allow(dead_code))]
fn valid_unit(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".service")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'))
}

/// Parse `systemctl list-units --plain --no-legend` rows:
/// `UNIT  LOAD  ACTIVE  SUB  DESCRIPTION…`
#[cfg_attr(windows, allow(dead_code))]
fn parse_units(text: &str) -> Vec<Service> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let mut it = t.split_whitespace();
        let name = match it.next() {
            Some(n) if n.ends_with(".service") => n,
            _ => continue,
        };
        let _load = it.next().unwrap_or("");
        let active = it.next().unwrap_or("").to_string();
        let sub = it.next().unwrap_or("").to_string();
        let description = it.collect::<Vec<_>>().join(" ");
        out.push(Service {
            name: name.to_string(),
            active,
            sub,
            description,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{parse_units, valid_unit};

    #[test]
    fn parses_unit_rows() {
        let s = "\
bluetooth.service loaded active running Bluetooth service
NetworkManager.service loaded active running Network Manager
cups.service loaded inactive dead CUPS Scheduler
not-a-unit.mount loaded active mounted nope
";
        let v = parse_units(s);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].name, "bluetooth.service");
        assert_eq!(v[0].active, "active");
        assert_eq!(v[0].sub, "running");
        assert_eq!(v[0].description, "Bluetooth service");
        assert_eq!(v[2].active, "inactive");
    }

    #[test]
    fn validates_unit_names() {
        assert!(valid_unit("NetworkManager.service"));
        assert!(valid_unit("getty@tty1.service"));
        assert!(!valid_unit("evil; rm -rf.service"));
        assert!(!valid_unit("foo.mount"));
    }
}
