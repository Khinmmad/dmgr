mod audio;
mod backend;
mod bluetooth;
mod commands;
mod details;
mod hotplug;
mod kernel;
mod platform;
mod power;
mod privileged;
mod system;
#[cfg(target_os = "windows")]
mod winutil;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::try_init();

    // Apply GPU/session-specific rendering workarounds (Nvidia+Wayland only).
    platform::apply_webkit_workarounds();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(backend::current_backend())
        .setup(|app| {
            hotplug::spawn(app.handle().clone());
            bluetooth::spawn_monitor(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_devices,
            commands::get_available_drivers,
            commands::get_property,
            commands::set_property,
            commands::bind_driver,
            commands::unbind_driver,
            commands::reload_driver,
            commands::set_device_enabled,
            commands::audio_outputs,
            commands::audio_inputs,
            commands::audio_set_default_output,
            commands::audio_set_default_input,
            commands::audio_set_volume,
            commands::audio_set_mute,
            commands::audio_app_streams,
            commands::audio_set_app_volume,
            commands::audio_set_app_mute,
            commands::bt_state,
            commands::bt_pair,
            commands::bt_connect,
            commands::bt_disconnect,
            commands::bt_set_power,
            commands::bt_set_trust,
            commands::bt_remove,
            commands::bt_scan,
            commands::capabilities,
            commands::platform_info,
            commands::system_info,
            commands::power_info,
            commands::power_set_profile,
            commands::power_set_brightness,
            commands::advanced_details,
            commands::kernel_modules,
            commands::kernel_module_info,
            commands::kernel_module_load,
            commands::kernel_module_unload,
            commands::open_device_manager,
            commands::open_bluetooth_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running dmgr-desktop");
}
