//! Kernel module management — list (/proc/modules), info (modinfo), load/unload
//! (modprobe via the privileged path). Linux-only; something Windows can't do.

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

/// Parse `/proc/modules`: "name size refcount used_by state address".
pub fn list() -> Vec<KernelModule> {
    let text = std::fs::read_to_string("/proc/modules").unwrap_or_default();
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
    privileged::run_pkexec("modprobe", &[name])
}

pub fn unload(name: &str) -> Result<(), String> {
    validate(name)?;
    privileged::run_pkexec("modprobe", &["-r", name])
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
