//! Windows audio backend via Core Audio (MMDevice enumeration + WASAPI endpoint
//! volume + the undocumented IPolicyConfig for switching the default device).
//!
//! Mirrors the Linux backends' `AudioBackend` contract: the opaque `AudioDevice.name`
//! is the MMDevice endpoint id string (what `GetId`/`GetDevice` use).

// The IPolicyConfig methods keep their Windows COM names (not snake_case) and most
// are unused vtable placeholders.
#![allow(non_snake_case)]

use super::{detect_kind, AudioBackend, AudioDevice};
use std::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IMMDevice,
    IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

/// CLSID_CPolicyConfigClient — the COM server that switches the default endpoint.
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

/// Undocumented IPolicyConfig. Only `SetDefaultEndpoint` (vtable slot 10) is used;
/// the earlier methods are declared solely to keep the vtable layout correct and
/// are never called, so their parameters are opaque pointers.
#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    unsafe fn GetMixFormat(&self, name: PCWSTR, format: *mut *mut c_void) -> HRESULT;
    unsafe fn GetDeviceFormat(&self, name: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
    unsafe fn ResetDeviceFormat(&self, name: PCWSTR) -> HRESULT;
    unsafe fn SetDeviceFormat(&self, name: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    unsafe fn GetProcessingPeriod(&self, name: PCWSTR, default: i32, def: *mut i64, min: *mut i64) -> HRESULT;
    unsafe fn SetProcessingPeriod(&self, name: PCWSTR, period: *mut i64) -> HRESULT;
    unsafe fn GetShareMode(&self, name: PCWSTR, mode: *mut c_void) -> HRESULT;
    unsafe fn SetShareMode(&self, name: PCWSTR, mode: *mut c_void) -> HRESULT;
    unsafe fn GetPropertyValue(&self, name: PCWSTR, store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
    unsafe fn SetPropertyValue(&self, name: PCWSTR, store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
    unsafe fn SetDefaultEndpoint(&self, name: PCWSTR, role: ERole) -> HRESULT;
    unsafe fn SetEndpointVisibility(&self, name: PCWSTR, visible: i32) -> HRESULT;
}

pub struct Wasapi;

impl AudioBackend for Wasapi {
    fn name(&self) -> &'static str {
        "wasapi"
    }

    fn outputs(&self) -> Vec<AudioDevice> {
        list(eRender).unwrap_or_default()
    }

    fn inputs(&self) -> Vec<AudioDevice> {
        list(eCapture).unwrap_or_default()
    }

    fn set_default_output(&self, id: &str) -> Result<(), String> {
        set_default(id)
    }

    fn set_default_input(&self, id: &str) -> Result<(), String> {
        set_default(id)
    }

    fn set_volume(&self, id: &str, percent: u32) -> Result<(), String> {
        run(|| unsafe {
            let dev = device_by_id(id)?;
            let epv: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None)?;
            let level = (percent.min(100) as f32) / 100.0;
            epv.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
            Ok(())
        })
    }

    fn set_mute(&self, id: &str, muted: bool) -> Result<(), String> {
        run(|| unsafe {
            let dev = device_by_id(id)?;
            let epv: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None)?;
            epv.SetMute(muted, std::ptr::null())?;
            Ok(())
        })
    }
}

// ── COM helpers ───────────────────────────────────────────────────────────────

/// Ensure COM is initialised on this thread, then run `f`, mapping errors to String.
fn run<T>(f: impl FnOnce() -> windows::core::Result<T>) -> Result<T, String> {
    unsafe {
        // S_FALSE (already initialised) and RPC_E_CHANGED_MODE (thread is STA) are
        // both fine — COM is usable either way. Never uninitialise here.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f().map_err(|e| e.message().to_string())
}

unsafe fn enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
}

unsafe fn device_by_id(id: &str) -> windows::core::Result<IMMDevice> {
    let wide = wide(id);
    enumerator()?.GetDevice(PCWSTR(wide.as_ptr()))
}

/// Enumerate active endpoints for a data flow into `AudioDevice`s.
fn list(flow: EDataFlow) -> Result<Vec<AudioDevice>, String> {
    run(|| unsafe {
        let en = enumerator()?;
        let default_id = en
            .GetDefaultAudioEndpoint(flow, eConsole)
            .ok()
            .map(|d| id_of(&d))
            .unwrap_or_default();

        let collection = en.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let dev = match collection.Item(i) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let id = id_of(&dev);
            let description = friendly_name(&dev).unwrap_or_else(|| id.clone());

            // Volume / mute are best-effort; a device may refuse activation.
            let (volume, muted) = match dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                Ok(epv) => {
                    let v = epv
                        .GetMasterVolumeLevelScalar()
                        .map(|s| (s * 100.0).round() as u32)
                        .ok();
                    let m = epv.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                    (v, m)
                }
                Err(_) => (None, false),
            };

            let kind = detect_kind(&description).to_string();
            out.push(AudioDevice {
                index: i,
                is_default: !id.is_empty() && id == default_id,
                name: id,
                description,
                state: String::new(),
                muted,
                volume,
                kind,
            });
        }
        Ok(out)
    })
}

/// Switch the default endpoint for all three roles (Console/Multimedia/Comms).
fn set_default(id: &str) -> Result<(), String> {
    run(|| unsafe {
        let config: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)?;
        let wide = wide(id);
        let pc = PCWSTR(wide.as_ptr());
        for role in [eConsole, eMultimedia, eCommunications] {
            config.SetDefaultEndpoint(pc, role).ok()?;
        }
        Ok(())
    })
}

/// MMDevice endpoint id string (`GetId` returns CoTaskMem memory we must free).
unsafe fn id_of(dev: &IMMDevice) -> String {
    match dev.GetId() {
        Ok(p) => pwstr_take(p),
        Err(_) => String::new(),
    }
}

/// Friendly display name from the device property store. `PROPVARIANT`'s Display
/// impl converts the value (a VT_LPWSTR here) to a string and frees on drop.
unsafe fn friendly_name(dev: &IMMDevice) -> Option<String> {
    let store = dev.OpenPropertyStore(STGM_READ).ok()?;
    let prop = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
    let s = prop.to_string();
    (!s.is_empty()).then_some(s)
}

/// Read a CoTaskMem-allocated wide string into a `String` and free it.
unsafe fn pwstr_take(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let s = p.to_string().unwrap_or_default();
    CoTaskMemFree(Some(p.0 as *const c_void));
    s
}

/// NUL-terminated UTF-16 buffer for passing &str as PCWSTR.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
