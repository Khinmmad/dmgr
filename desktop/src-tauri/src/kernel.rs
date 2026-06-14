//! Kernel module management — list (/proc/modules), info (modinfo), load/unload
//! (modprobe via the privileged path). Linux-only; something Windows can't do.

#[cfg(not(windows))]
use crate::privileged;
use serde::Serialize;

#[derive(Serialize)]
pub struct KernelModule {
    pub name: String,
    pub size_kb: u64,
    pub refcount: i32,
    pub used_by: Vec<String>,
    pub state: String, // Live | Loading | Unloading
}

#[derive(Serialize)]
pub struct ModuleInfo {
    pub name: String,
    pub filename: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub version: Option<String>,
    pub depends: Vec<String>,
    pub params: Vec<String>,
}

/// Read and parse `/proc/modules`.
pub fn list() -> Vec<KernelModule> {
    let text = std::fs::read_to_string("/proc/modules").unwrap_or_default();
    parse_proc_modules(&text)
}

/// Parse `/proc/modules`: "name size refcount used_by state address".
fn parse_proc_modules(text: &str) -> Vec<KernelModule> {
    let mut mods = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 5 {
            continue;
        }
        let used_by: Vec<String> = if f[3] == "-" {
            Vec::new()
        } else {
            f[3].split(',').filter(|s| !s.is_empty()).map(String::from).collect()
        };
        mods.push(KernelModule {
            name: f[0].to_string(),
            size_kb: f[1].parse::<u64>().unwrap_or(0) / 1024,
            refcount: f[2].parse().unwrap_or(0),
            used_by,
            state: f[4].to_string(),
        });
    }
    mods.sort_by(|a, b| a.name.cmp(&b.name));
    mods
}

pub fn info(name: &str) -> ModuleInfo {
    let text = run_modinfo(name).unwrap_or_default();
    let field = |key: &str| -> Option<String> {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
    };
    let multi = |key: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| {
                let (k, v) = l.split_once(':')?;
                (k.trim() == key).then(|| v.trim().to_string())
            })
            .collect()
    };
    let depends = field("depends")
        .map(|d| d.split(',').filter(|s| !s.is_empty()).map(String::from).collect())
        .unwrap_or_default();

    ModuleInfo {
        name: name.to_string(),
        filename: field("filename"),
        description: field("description"),
        author: field("author"),
        license: field("license"),
        version: field("version"),
        depends,
        params: multi("parm"),
    }
}

pub fn load(name: &str) -> Result<(), String> {
    validate(name)?;
    #[cfg(not(windows))]
    {
        privileged::run_pkexec("modprobe", &[name])
    }
    #[cfg(windows)]
    {
        Err("kernel modules are Linux-only".into())
    }
}

pub fn unload(name: &str) -> Result<(), String> {
    validate(name)?;
    #[cfg(not(windows))]
    {
        privileged::run_pkexec("modprobe", &["-r", name])
    }
    #[cfg(windows)]
    {
        Err("kernel modules are Linux-only".into())
    }
}

fn validate(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("invalid module name".into());
    }
    Ok(())
}

fn run_modinfo(name: &str) -> Option<String> {
    let out = std::process::Command::new("modinfo").arg(name).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_modules() {
        let sample = "\
nvidia_drm 122880 12 - Live 0x0000000000000000
nvidia_modeset 1572864 6 nvidia_drm, Live 0x0000000000000000
btrfs 1830912 1 - Live 0x0000000000000000
videodev 376832 4 uvcvideo,videobuf2_v4l2 Live 0x0000000000000000
";
        let mods = parse_proc_modules(sample);
        assert_eq!(mods.len(), 4);

        // sorted by name → btrfs first
        assert_eq!(mods[0].name, "btrfs");
        assert_eq!(mods[0].refcount, 1);
        assert!(mods[0].used_by.is_empty());

        let drm = mods.iter().find(|m| m.name == "nvidia_drm").unwrap();
        assert_eq!(drm.refcount, 12);
        assert_eq!(drm.size_kb, 122880 / 1024);
        assert_eq!(drm.state, "Live");

        let videodev = mods.iter().find(|m| m.name == "videodev").unwrap();
        assert_eq!(videodev.used_by, vec!["uvcvideo", "videobuf2_v4l2"]);
    }

    #[test]
    fn ignores_malformed_lines() {
        assert!(parse_proc_modules("garbage\n\nx y\n").is_empty());
    }
}
