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

#[cfg(not(target_os = "linux"))]
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
