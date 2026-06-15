//! Read-only system overview: kernel, CPU, memory, uptime, load.
//! Linux reads /proc; other platforms return a minimal stub (the panel is
//! Linux-gated in the UI).

use serde::Serialize;

#[derive(Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel: String,
    pub arch: String,
    pub uptime: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub mem_total_mb: u64,
    pub mem_used_mb: u64,
    pub load_avg: String,
}

#[cfg(target_os = "linux")]
pub fn info() -> SystemInfo {
    let total_kb = parse_meminfo(&read("/proc/meminfo"), "MemTotal:");
    let avail_kb = parse_meminfo(&read("/proc/meminfo"), "MemAvailable:");
    SystemInfo {
        hostname: read("/proc/sys/kernel/hostname").trim().to_string(),
        kernel: read("/proc/sys/kernel/osrelease").trim().to_string(),
        arch: std::env::consts::ARCH.to_string(),
        uptime: fmt_uptime(parse_uptime(&read("/proc/uptime"))),
        cpu_model: parse_cpu_model(&read("/proc/cpuinfo")),
        cpu_cores: parse_cpu_cores(&read("/proc/cpuinfo")),
        mem_total_mb: total_kb / 1024,
        mem_used_mb: total_kb.saturating_sub(avail_kb) / 1024,
        load_avg: parse_loadavg(&read("/proc/loadavg")),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn info() -> SystemInfo {
    SystemInfo {
        hostname: String::new(),
        kernel: String::new(),
        arch: std::env::consts::ARCH.to_string(),
        uptime: String::new(),
        cpu_model: String::new(),
        cpu_cores: 0,
        mem_total_mb: 0,
        mem_used_mb: 0,
        load_avg: String::new(),
    }
}

#[cfg(target_os = "linux")]
fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// ── pure parsers (unit-tested) ──────────────────────────────────────────────

/// `MemTotal:   16331756 kB` → 16331756 (kB).
fn parse_meminfo(text: &str, key: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix(key))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0)
}

/// First field of /proc/uptime (seconds, fractional) → whole seconds.
fn parse_uptime(text: &str) -> u64 {
    text.split('.').next().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0)
}

fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

/// `model name\t: AMD Ryzen 9 ...` → the value.
fn parse_cpu_model(text: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix("model name"))
        .map(|v| v.trim().trim_start_matches(':').trim().to_string())
        .unwrap_or_default()
}

fn parse_cpu_cores(text: &str) -> u32 {
    text.lines().filter(|l| l.starts_with("processor")).count() as u32
}

/// `0.52 0.48 0.40 1/1234 56789` → `0.52 0.48 0.40`.
fn parse_loadavg(text: &str) -> String {
    text.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meminfo_and_cpu() {
        let mem = "MemTotal:       16331756 kB\nMemFree: 100 kB\nMemAvailable:    8000000 kB\n";
        assert_eq!(parse_meminfo(mem, "MemTotal:"), 16331756);
        assert_eq!(parse_meminfo(mem, "MemAvailable:"), 8000000);
        assert_eq!(parse_meminfo(mem, "Nope:"), 0);

        let cpu = "processor\t: 0\nmodel name\t: AMD Ryzen 9 5900X\nprocessor\t: 1\nmodel name\t: AMD Ryzen 9 5900X\n";
        assert_eq!(parse_cpu_model(cpu), "AMD Ryzen 9 5900X");
        assert_eq!(parse_cpu_cores(cpu), 2);
    }

    #[test]
    fn uptime_and_load() {
        assert_eq!(parse_uptime("123456.78 98765.43"), 123456);
        assert_eq!(fmt_uptime(90), "1m");
        assert_eq!(fmt_uptime(3700), "1h 1m");
        assert_eq!(fmt_uptime(90061), "1d 1h 1m");
        assert_eq!(parse_loadavg("0.52 0.48 0.40 1/1234 56789"), "0.52 0.48 0.40");
    }
}
