use dmgr_core::device::Device;
use dmgr_core::error::DmgrError;
use dmgr_core::sysfs;
use dmgr_core::udev::{UdevEvent, UdevMonitor};
use dmgr_core::{control, properties};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::zvariant::ObjectPath;

#[derive(Clone)]
struct DeviceStore {
    devices: Arc<Mutex<HashMap<String, Device>>>,
}

impl DeviceStore {
    fn new() -> Self {
        DeviceStore {
            devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn scan(&self) -> Result<usize, DmgrError> {
        let devices = sysfs::scan_all_devices()?;
        let mut map = self.devices.lock().unwrap();
        let count = devices.len();
        for device in devices {
            map.insert(device.id.clone(), device);
        }
        info!("Scan: {} devices loaded", count);
        Ok(count)
    }

    fn get_all(&self) -> Vec<Device> {
        let map = self.devices.lock().unwrap();
        map.values().cloned().collect()
    }

    fn get(&self, id: &str) -> Option<Device> {
        let map = self.devices.lock().unwrap();
        map.get(id).cloned()
    }

    fn get_by_bus(&self, bus: &str) -> Vec<Device> {
        let map = self.devices.lock().unwrap();
        map.values()
            .filter(|d| d.bus.to_string().to_lowercase() == bus.to_lowercase())
            .cloned()
            .collect()
    }

    fn search(&self, query: &str) -> Vec<Device> {
        let q = query.to_lowercase();
        let map = self.devices.lock().unwrap();
        map.values()
            .filter(|d| {
                d.name.to_lowercase().contains(&q)
                    || d.id.to_lowercase().contains(&q)
                    || d.vendor.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || d.model.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || d.driver.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    fn update_device(&self, dev_id: &str) {
        let path = {
            let map = self.devices.lock().unwrap();
            map.get(dev_id).map(|d| d.path.clone())
        };

        if let Some(path) = path {
            if let Ok(devices) = sysfs::scan_all_devices() {
                let mut map = self.devices.lock().unwrap();
                for device in devices {
                    if device.path == path || device.id == dev_id {
                        map.insert(dev_id.to_string(), device);
                        return;
                    }
                }
            }
        }

        warn!("Could not refresh device {}", dev_id);
    }
}

struct DeviceManager {
    store: DeviceStore,
}

#[zbus::interface(name = "org.dmgr.DeviceManager")]
impl DeviceManager {
    async fn get_all_devices(&self) -> String {
        let devices = self.store.get_all();
        serde_json::to_string(&devices).unwrap_or_else(|e| {
            error!("JSON error: {}", e);
            "[]".to_string()
        })
    }

    async fn get_device(&self, dev_id: &str) -> String {
        match self.store.get(dev_id) {
            Some(device) => serde_json::to_string(&device).unwrap_or_else(|e| {
                error!("JSON error: {}", e);
                "{}".to_string()
            }),
            None => "{}".to_string(),
        }
    }

    async fn get_devices_by_bus(&self, bus: &str) -> String {
        let devices = self.store.get_by_bus(bus);
        serde_json::to_string(&devices).unwrap_or_else(|e| {
            error!("JSON error: {}", e);
            "[]".to_string()
        })
    }

    async fn get_devices_by_filter(&self, query: &str) -> String {
        let devices = self.store.search(query);
        serde_json::to_string(&devices).unwrap_or_else(|e| {
            error!("JSON error: {}", e);
            "[]".to_string()
        })
    }

    async fn get_available_drivers(&self, dev_id: &str) -> Vec<String> {
        let device = self.store.get(dev_id);
        match device {
            Some(d) => properties::get_available_drivers(&d.path).unwrap_or_default(),
            None => vec![],
        }
    }

    async fn bind_driver(&self, dev_id: &str, driver: &str) -> bool {
        let device = self.store.get(dev_id);
        match device {
            Some(d) => match control::bind_driver(&d.path, driver) {
                Ok(()) => {
                    info!("Bound device {} to driver {}", dev_id, driver);
                    self.store.update_device(dev_id);
                    true
                }
                Err(e) => {
                    error!("Bind failed: {}", e);
                    false
                }
            },
            None => false,
        }
    }

    async fn unbind_driver(&self, dev_id: &str) -> bool {
        let device = self.store.get(dev_id);
        match device {
            Some(d) => match control::unbind_driver(&d.path) {
                Ok(()) => {
                    info!("Unbound device {}", dev_id);
                    self.store.update_device(dev_id);
                    true
                }
                Err(e) => {
                    error!("Unbind failed: {}", e);
                    false
                }
            },
            None => false,
        }
    }

    async fn set_property(&self, dev_id: &str, attr: &str, value: &str) -> bool {
        let device = self.store.get(dev_id);
        match device {
            Some(d) => match properties::set_property(&d.path, attr, value) {
                Ok(()) => {
                    info!("Set property {}={} on {}", attr, value, dev_id);
                    self.store.update_device(dev_id);
                    true
                }
                Err(e) => {
                    error!("Set property failed: {}", e);
                    false
                }
            },
            None => false,
        }
    }

    async fn refresh(&self) -> u32 {
        let count = self.store.scan().unwrap_or(0) as u32;
        info!("Manual refresh: {} devices", count);
        count
    }

    #[zbus(signal)]
    async fn device_added(signal_ctxt: &zbus::SignalContext<'_>, count: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn device_removed(signal_ctxt: &zbus::SignalContext<'_>, count: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn device_changed(signal_ctxt: &zbus::SignalContext<'_>, dev_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn scan_finished(signal_ctxt: &zbus::SignalContext<'_>, count: u32) -> zbus::Result<()>;
}

async fn run_udev_monitor(store: DeviceStore, conn: zbus::Connection) {
    let mut monitor = UdevMonitor::new();

    let rx = match monitor.start() {
        Ok(rx) => {
            info!("Udev monitor thread started");
            rx
        }
        Err(e) => {
            error!("Failed to start udev monitor: {}", e);
            return;
        }
    };

    for event in rx {
        match event {
            UdevEvent::DeviceAdded(_) => {
                info!("udev: device added");
                let count = store.scan().unwrap_or(0) as u32;
                let _ = emit_signal_device_added(&conn, count).await;
            }
            UdevEvent::DeviceRemoved(_) => {
                info!("udev: device removed");
                let count = store.scan().unwrap_or(0) as u32;
                let _ = emit_signal_device_removed(&conn, count).await;
            }
            UdevEvent::DeviceChanged(devpath) => {
                info!("udev: device changed {}", devpath);
                let _ = store.scan();
                let _ = emit_signal_device_changed(&conn, &devpath).await;
            }
            UdevEvent::ScanComplete => {
                info!("Manual scan triggered");
                let _ = store.scan();
                let count = store.get_all().len() as u32;
                let _ = emit_signal_scan_finished(&conn, count).await;
            }
        }
    }
}

async fn emit_signal_device_added(conn: &zbus::Connection, count: u32) -> zbus::Result<()> {
    let path: ObjectPath = "/org/dmgr/DeviceManager".try_into().unwrap();
    let ctx = zbus::SignalContext::new(conn, path)?;
    DeviceManager::device_added(&ctx, count).await
}

async fn emit_signal_device_removed(conn: &zbus::Connection, count: u32) -> zbus::Result<()> {
    let path: ObjectPath = "/org/dmgr/DeviceManager".try_into().unwrap();
    let ctx = zbus::SignalContext::new(conn, path)?;
    DeviceManager::device_removed(&ctx, count).await
}

async fn emit_signal_device_changed(conn: &zbus::Connection, dev_id: &str) -> zbus::Result<()> {
    let path: ObjectPath = "/org/dmgr/DeviceManager".try_into().unwrap();
    let ctx = zbus::SignalContext::new(conn, path)?;
    DeviceManager::device_changed(&ctx, dev_id).await
}

async fn emit_signal_scan_finished(conn: &zbus::Connection, count: u32) -> zbus::Result<()> {
    let path: ObjectPath = "/org/dmgr/DeviceManager".try_into().unwrap();
    let ctx = zbus::SignalContext::new(conn, path)?;
    DeviceManager::scan_finished(&ctx, count).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting dmgr-daemon...");

    let store = DeviceStore::new();
    let count = store.scan().map_err(|e| format!("Initial scan failed: {}", e))?;
    info!("Initial scan complete: {} devices", count);

    let manager = DeviceManager {
        store: store.clone(),
    };

    let conn_result = zbus::ConnectionBuilder::session()?
        .name("org.dmgr.DeviceManager")
        .map_err(|e| {
            error!("DBus name registration failed: {}", e);
            e
        })?
        .serve_at("/org/dmgr/DeviceManager", manager)
        .map_err(|e| {
            error!("Failed to serve object: {}", e);
            e
        })?
        .build()
        .await;

    let conn = match conn_result {
        Ok(c) => c,
        Err(zbus::Error::NameTaken) => {
            eprintln!("dmgr-daemon is already running (DBus name 'org.dmgr.DeviceManager' is taken).");
            eprintln!("Stop the existing instance first:");
            eprintln!("  systemctl --user stop dmgr-daemon");
            eprintln!("  or kill the existing dmgr-daemon process.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to start dmgr-daemon: {}", e);
            std::process::exit(1);
        }
    };

    let monitor_store = store.clone();
    let monitor_conn = conn.clone();
    tokio::spawn(async move {
        run_udev_monitor(monitor_store, monitor_conn).await;
    });

    info!("dmgr-daemon ready on session bus");

    std::future::pending::<()>().await;
    Ok(())
}
