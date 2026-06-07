mod audio;
mod bluetooth;
mod commands;
mod privileged;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::try_init();

    // WebKitGTK renders a blank window under Nvidia + Wayland with the DMABUF
    // renderer. Disable it unless the user has set their own preference.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_devices,
            commands::get_available_drivers,
            commands::get_property,
            commands::set_property,
            commands::bind_driver,
            commands::unbind_driver,
            commands::set_device_enabled,
            commands::audio_outputs,
            commands::audio_inputs,
            commands::audio_set_default_output,
            commands::audio_set_default_input,
            commands::audio_set_volume,
            commands::audio_set_mute,
            commands::bt_state,
            commands::bt_connect,
            commands::bt_disconnect,
            commands::bt_set_power,
            commands::bt_set_trust,
            commands::capabilities,
        ])
        .run(tauri::generate_context!())
        .expect("error while running dmgr-desktop");
}
