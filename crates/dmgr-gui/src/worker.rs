use crate::audio;
use dmgr_core::device::Device;
use dmgr_core::error::DmgrError;
use dmgr_core::udev::{UdevEvent, UdevMonitor};
use dmgr_core::{control, properties, sysfs};
use log::{error, info, warn};
use std::sync::mpsc;

// ── Messages: UI → Worker ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum WorkerMsg {
    // Device management
    Refresh,
    GetDrivers { dev_id: String, path: String },
    BindDriver { dev_id: String, path: String, driver: String },
    UnbindDriver { dev_id: String, path: String },
    SetProperty { dev_id: String, path: String, attr: String, value: String },

    // Audio management
    RefreshAudio,
    SetDefaultSink(String),
    SetDefaultSource(String),
}

// ── Results: Worker → UI ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum BgResult {
    Scanned(Vec<Device>),
    DriversLoaded { dev_id: String, drivers: Vec<String> },
    OpResult { success: bool, message: String, dev_id: Option<String> },
    UdevTriggered,
    AudioDevices {
        sinks: Vec<audio::AudioDevice>,
        sources: Vec<audio::AudioDevice>,
    },
    AudioResult { success: bool, message: String },
}

// ── Worker thread ─────────────────────────────────────────────────────────────

pub fn start_worker(msg_rx: mpsc::Receiver<WorkerMsg>, result_tx: mpsc::Sender<BgResult>) {
    std::thread::spawn(move || {
        info!("Worker thread started");
        for msg in msg_rx {
            match msg {
                // Device
                WorkerMsg::Refresh => handle_refresh(&result_tx),
                WorkerMsg::GetDrivers { dev_id, path } => {
                    handle_get_drivers(dev_id, path, &result_tx);
                }
                WorkerMsg::BindDriver { dev_id, path, driver } => {
                    handle_bind(dev_id, path, driver, &result_tx);
                }
                WorkerMsg::UnbindDriver { dev_id, path } => {
                    handle_unbind(dev_id, path, &result_tx);
                }
                WorkerMsg::SetProperty { dev_id, path, attr, value } => {
                    handle_set_property(dev_id, path, attr, value, &result_tx);
                }

                // Audio
                WorkerMsg::RefreshAudio => {
                    let sinks = audio::list_sinks();
                    let sources = audio::list_sources();
                    let _ = result_tx.send(BgResult::AudioDevices { sinks, sources });
                }
                WorkerMsg::SetDefaultSink(name) => {
                    let result = audio::set_default_sink(&name);
                    let (success, message) = match &result {
                        Ok(()) => (true, format!("Output switched to: {}", name)),
                        Err(e) => (false, e.clone()),
                    };
                    // Refresh audio state after switching
                    let sinks = audio::list_sinks();
                    let sources = audio::list_sources();
                    let _ = result_tx.send(BgResult::AudioDevices { sinks, sources });
                    let _ = result_tx.send(BgResult::AudioResult { success, message });
                }
                WorkerMsg::SetDefaultSource(name) => {
                    let result = audio::set_default_source(&name);
                    let (success, message) = match &result {
                        Ok(()) => (true, format!("Input switched to: {}", name)),
                        Err(e) => (false, e.clone()),
                    };
                    let sinks = audio::list_sinks();
                    let sources = audio::list_sources();
                    let _ = result_tx.send(BgResult::AudioDevices { sinks, sources });
                    let _ = result_tx.send(BgResult::AudioResult { success, message });
                }
            }
        }
        info!("Worker thread exiting");
    });
}

pub fn start_udev_monitor(result_tx: mpsc::Sender<BgResult>) {
    std::thread::spawn(move || {
        let mut monitor = UdevMonitor::new();
        match monitor.start() {
            Ok(rx) => {
                info!("Udev monitor started");
                for event in rx {
                    match event {
                        UdevEvent::DeviceAdded(_)
                        | UdevEvent::DeviceRemoved(_)
                        | UdevEvent::DeviceChanged(_)
                        | UdevEvent::ScanComplete => {
                            let _ = result_tx.send(BgResult::UdevTriggered);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Could not start udev monitor: {}", e);
            }
        }
    });
}

// ── Device handlers ───────────────────────────────────────────────────────────

fn handle_refresh(tx: &mpsc::Sender<BgResult>) {
    info!("Worker: scanning all devices");
    match sysfs::scan_all_devices() {
        Ok(devices) => {
            let _ = tx.send(BgResult::Scanned(devices));
        }
        Err(e) => {
            error!("Scan failed: {}", e);
            let _ = tx.send(BgResult::OpResult {
                success: false,
                message: format!("Scan failed: {}", e),
                dev_id: None,
            });
        }
    }
}

fn handle_get_drivers(dev_id: String, path: String, tx: &mpsc::Sender<BgResult>) {
    let drivers = properties::get_available_drivers(&path).unwrap_or_default();
    let _ = tx.send(BgResult::DriversLoaded { dev_id, drivers });
}

fn handle_bind(dev_id: String, path: String, driver: String, tx: &mpsc::Sender<BgResult>) {
    let result = privileged(|| control::bind_driver(&path, &driver), &["bind", &path, &driver]);
    if result.is_ok() {
        if let Ok(devices) = sysfs::scan_all_devices() {
            let _ = tx.send(BgResult::Scanned(devices));
        }
    }
    let (success, message) = match result {
        Ok(()) => (true, format!("Driver '{}' bound successfully", driver)),
        Err(e) => (false, e),
    };
    let _ = tx.send(BgResult::OpResult { success, message, dev_id: Some(dev_id) });
}

fn handle_unbind(dev_id: String, path: String, tx: &mpsc::Sender<BgResult>) {
    let result = privileged(|| control::unbind_driver(&path), &["unbind", &path]);
    if result.is_ok() {
        if let Ok(devices) = sysfs::scan_all_devices() {
            let _ = tx.send(BgResult::Scanned(devices));
        }
    }
    let (success, message) = match result {
        Ok(()) => (true, "Driver unbound successfully".to_string()),
        Err(e) => (false, e),
    };
    let _ = tx.send(BgResult::OpResult { success, message, dev_id: Some(dev_id) });
}

fn handle_set_property(
    dev_id: String,
    path: String,
    attr: String,
    value: String,
    tx: &mpsc::Sender<BgResult>,
) {
    let result =
        privileged(|| properties::set_property(&path, &attr, &value), &["set", &path, &attr, &value]);
    if result.is_ok() {
        if let Ok(devices) = sysfs::scan_all_devices() {
            let _ = tx.send(BgResult::Scanned(devices));
        }
    }
    let (success, message) = match result {
        Ok(()) => (true, format!("Property '{}' set to '{}'", attr, value)),
        Err(e) => (false, e),
    };
    let _ = tx.send(BgResult::OpResult { success, message, dev_id: Some(dev_id) });
}

// ── Privilege escalation ──────────────────────────────────────────────────────

fn privileged<F>(direct: F, pkexec_args: &[&str]) -> Result<(), String>
where
    F: FnOnce() -> dmgr_core::error::Result<()>,
{
    match direct() {
        Ok(()) => Ok(()),
        Err(DmgrError::PermissionDenied(_)) => pkexec(pkexec_args),
        Err(e) => Err(e.to_string()),
    }
}

fn pkexec(args: &[&str]) -> Result<(), String> {
    info!("Escalating via pkexec: {:?}", args);
    let status = std::process::Command::new("pkexec")
        .arg("dmgr-polkit-helper")
        .args(args)
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!(
            "pkexec dmgr-polkit-helper failed (exit {}). \
             Install dmgr-polkit-helper to /usr/bin for privilege escalation.",
            s.code().unwrap_or(-1)
        )),
        Err(e) => Err(format!("Could not run pkexec: {}", e)),
    }
}
