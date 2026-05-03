use crate::error::{DmgrError, Result};
use log::{debug, info, warn};
use std::sync::mpsc;
use std::thread;

pub struct UdevMonitor {
    tx: Option<mpsc::Sender<UdevEvent>>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub enum UdevEvent {
    DeviceAdded(String),
    DeviceRemoved(String),
    DeviceChanged(String),
    ScanComplete,
}

impl UdevMonitor {
    pub fn new() -> Self {
        UdevMonitor {
            tx: None,
            handle: None,
        }
    }

    pub fn start(&mut self) -> Result<mpsc::Receiver<UdevEvent>> {
        let (tx, rx) = mpsc::channel();
        self.tx = Some(tx.clone());

        let handle = thread::spawn(move || match monitor_udev_events(tx) {
            Ok(_) => info!("Udev monitor thread finished"),
            Err(e) => warn!("Udev monitor error: {}", e),
        });

        self.handle = Some(handle);
        Ok(rx)
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            drop(handle);
        }
        self.tx = None;
    }

    pub fn refresh(&self) -> Result<()> {
        if let Some(ref tx) = self.tx {
            tx.send(UdevEvent::ScanComplete)
                .map_err(|e| DmgrError::Udev(format!("Channel closed: {}", e)))
        } else {
            Err(DmgrError::Udev("Monitor not started".to_string()))
        }
    }
}

fn monitor_udev_events(tx: mpsc::Sender<UdevEvent>) -> Result<()> {
    let socket = udev::MonitorBuilder::new()
        .map_err(|e| DmgrError::Udev(format!("MonitorBuilder: {}", e)))?
        .match_subsystem_devtype("usb", "usb_device")
        .map_err(|e| DmgrError::Udev(format!("match usb: {}", e)))?
        .match_subsystem("pci")
        .map_err(|e| DmgrError::Udev(format!("match pci: {}", e)))?
        .match_subsystem("input")
        .map_err(|e| DmgrError::Udev(format!("match input: {}", e)))?
        .match_subsystem("sound")
        .map_err(|e| DmgrError::Udev(format!("match sound: {}", e)))?
        .match_subsystem("block")
        .map_err(|e| DmgrError::Udev(format!("match block: {}", e)))?
        .match_subsystem("drm")
        .map_err(|e| DmgrError::Udev(format!("match drm: {}", e)))?
        .match_subsystem("net")
        .map_err(|e| DmgrError::Udev(format!("match net: {}", e)))?
        .listen()
        .map_err(|e| DmgrError::Udev(format!("listen: {}", e)))?;

    info!("Udev monitor started");

    for event in socket.iter() {
        let devpath = event
            .device()
            .devpath()
            .to_string_lossy()
            .to_string();

        match event.event_type() {
            udev::EventType::Add => {
                debug!("udev add: {}", devpath);
                let _ = tx.send(UdevEvent::DeviceAdded(devpath));
            }
            udev::EventType::Remove => {
                debug!("udev remove: {}", devpath);
                let _ = tx.send(UdevEvent::DeviceRemoved(devpath));
            }
            udev::EventType::Change
            | udev::EventType::Bind
            | udev::EventType::Unbind => {
                debug!("udev change: {}", devpath);
                let _ = tx.send(UdevEvent::DeviceChanged(devpath));
            }
            _ => {}
        }
    }

    Ok(())
}

impl Drop for UdevMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}
