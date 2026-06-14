//! Live device hotplug monitor. Polls the udev netlink socket and emits a
//! `devices-changed` Tauri event so the frontend can re-scan automatically.

#[cfg(target_os = "linux")]
pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        if let Err(e) = run(app) {
            log::warn!("hotplug monitor stopped: {e}");
        }
    });
}

/// Windows: subscribe to device-interface arrival/removal via CfgMgr32
/// (CM_Register_Notification) and emit `devices-changed`. No window/message loop
/// needed — the callback runs on a system thread.
#[cfg(target_os = "windows")]
pub fn spawn(app: tauri::AppHandle) {
    use std::ffi::c_void;
    use std::sync::OnceLock;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Register_Notification, CM_NOTIFY_ACTION, CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL,
        CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL, CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER,
        CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
        CR_SUCCESS, HCMNOTIFICATION,
    };

    static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
    if APP.set(app).is_err() {
        return; // already registered
    }

    unsafe extern "system" fn callback(
        _h: HCMNOTIFICATION,
        _ctx: *const c_void,
        action: CM_NOTIFY_ACTION,
        _data: *const CM_NOTIFY_EVENT_DATA,
        _size: u32,
    ) -> u32 {
        if action == CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL
            || action == CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL
        {
            if let Some(app) = APP.get() {
                use tauri::Emitter;
                let _ = app.emit("devices-changed", ());
            }
        }
        0 // ERROR_SUCCESS
    }

    let mut filter = CM_NOTIFY_FILTER::default();
    filter.cbSize = std::mem::size_of::<CM_NOTIFY_FILTER>() as u32;
    filter.FilterType = CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE;
    filter.Flags = CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES;

    let mut handle = HCMNOTIFICATION::default();
    // The OS owns the subscription until unregister/exit; we never unregister.
    let ret = unsafe { CM_Register_Notification(&filter, None, Some(callback), &mut handle) };
    if ret == CR_SUCCESS {
        log::info!("windows hotplug monitor started");
    } else {
        log::warn!("CM_Register_Notification failed: {ret:?}");
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn spawn(_app: tauri::AppHandle) {}

#[cfg(target_os = "linux")]
fn run(app: tauri::AppHandle) -> Result<(), String> {
    use std::os::unix::io::AsRawFd;
    use tauri::Emitter;

    let socket = udev::MonitorBuilder::new()
        .map_err(|e| e.to_string())?
        .match_subsystem_devtype("usb", "usb_device")
        .map_err(|e| e.to_string())?
        .match_subsystem("pci")
        .map_err(|e| e.to_string())?
        .match_subsystem("input")
        .map_err(|e| e.to_string())?
        .match_subsystem("sound")
        .map_err(|e| e.to_string())?
        .match_subsystem("block")
        .map_err(|e| e.to_string())?
        .match_subsystem("drm")
        .map_err(|e| e.to_string())?
        .match_subsystem("net")
        .map_err(|e| e.to_string())?
        .listen()
        .map_err(|e| e.to_string())?;

    let fd = socket.as_raw_fd();
    log::info!("hotplug monitor started");

    loop {
        // Block (up to 1s) until the netlink socket has data; `iter()` is
        // non-blocking, so we must poll the fd ourselves.
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let rc = unsafe { libc::poll(&mut pfd, 1, 1000) };
        if rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(format!("poll failed: {err}"));
        }
        if rc == 0 {
            continue; // timeout, no events
        }

        let mut changed = false;
        for event in socket.iter() {
            use udev::EventType::*;
            match event.event_type() {
                Add | Remove | Bind | Unbind | Change => changed = true,
                _ => {}
            }
        }

        if changed {
            // Frontend debounces; one event per burst is enough.
            let _ = app.emit("devices-changed", ());
        }
    }
}
