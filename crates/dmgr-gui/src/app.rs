use crate::audio::{self, AudioDevice};
use crate::worker::{BgResult, WorkerMsg};
use dmgr_core::device::{Bus, Device, DeviceStatus};
use egui::{Color32, RichText, ScrollArea};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── Constants ─────────────────────────────────────────────────────────────────

const HIGHLIGHT_DURATION: Duration = Duration::from_secs(4);
const AUTO_REFRESH_SECS: u64 = 60;

// ── Confirm dialog ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ConfirmPending {
    title: String,
    body: String,
    action: WorkerMsg,
}

// ── Main App ──────────────────────────────────────────────────────────────────

pub struct DmgrApp {
    // Devices
    devices: Vec<Device>,
    selected_id: Option<String>,

    // Filtering
    search_query: String,
    bus_filter: String,
    show_all_devices: bool, // include TTY/noise

    // Driver UI
    available_drivers: Vec<String>,
    selected_driver: String,
    driver_custom: String,
    loading_drivers: bool,

    // Property editing
    prop_edits: HashMap<String, String>,

    // Background channels
    worker_tx: mpsc::Sender<WorkerMsg>,
    bg_rx: mpsc::Receiver<BgResult>,

    // Feedback
    status_msg: String,
    op_error: Option<String>,
    op_busy: bool,
    confirm: Option<ConfirmPending>,

    // Hotplug highlight: dev_id → time first seen (fades to normal)
    recent_adds: HashMap<String, Instant>,

    // Audio
    show_audio_window: bool,
    audio_sinks: Vec<AudioDevice>,
    audio_sources: Vec<AudioDevice>,
    audio_busy: bool,
    audio_error: Option<String>,
    audio_available: bool,

    // Stats
    udev_event_count: u32,
    last_scan: Instant,
    scan_count: usize,
}

impl DmgrApp {
    pub fn new() -> Self {
        let (worker_tx, worker_rx) = mpsc::channel::<WorkerMsg>();
        let (bg_tx, bg_rx) = mpsc::channel::<BgResult>();

        crate::worker::start_worker(worker_rx, bg_tx.clone());
        crate::worker::start_udev_monitor(bg_tx);

        let _ = worker_tx.send(WorkerMsg::Refresh);

        DmgrApp {
            devices: Vec::new(),
            selected_id: None,
            search_query: String::new(),
            bus_filter: String::new(),
            show_all_devices: false,
            available_drivers: Vec::new(),
            selected_driver: String::new(),
            driver_custom: String::new(),
            loading_drivers: false,
            prop_edits: HashMap::new(),
            worker_tx,
            bg_rx,
            status_msg: "Scanning devices…".to_string(),
            op_error: None,
            op_busy: true,
            confirm: None,
            recent_adds: HashMap::new(),
            show_audio_window: false,
            audio_sinks: Vec::new(),
            audio_sources: Vec::new(),
            audio_busy: false,
            audio_error: None,
            audio_available: audio::is_available(),
            udev_event_count: 0,
            last_scan: Instant::now(),
            scan_count: 0,
        }
    }

    // ── Background polling ─────────────────────────────────────────────────────

    fn poll_results(&mut self, ctx: &egui::Context) {
        let mut needs_repaint = false;

        while let Ok(result) = self.bg_rx.try_recv() {
            needs_repaint = true;
            match result {
                BgResult::Scanned(new_devices) => {
                    // Detect newly connected devices for hotplug highlighting
                    let old_ids: HashSet<&str> =
                        self.devices.iter().map(|d| d.id.as_str()).collect();
                    for d in &new_devices {
                        if !old_ids.contains(d.id.as_str()) {
                            self.recent_adds.insert(d.id.clone(), Instant::now());
                        }
                    }

                    self.scan_count = new_devices.len();
                    self.devices = new_devices;
                    self.last_scan = Instant::now();
                    self.op_busy = false;
                    self.status_msg =
                        format!("Scan complete — {} devices found", self.scan_count);

                    if let Some(ref id) = self.selected_id.clone() {
                        self.repopulate_prop_edits(id);
                    }
                }
                BgResult::DriversLoaded { dev_id, drivers } => {
                    if self.selected_id.as_deref() == Some(&dev_id) {
                        if self.selected_driver.is_empty() {
                            self.selected_driver =
                                drivers.first().cloned().unwrap_or_default();
                        }
                        self.available_drivers = drivers;
                        self.loading_drivers = false;
                    }
                }
                BgResult::OpResult { success, message, dev_id } => {
                    self.op_busy = false;
                    if success {
                        self.op_error = None;
                        self.status_msg = message;
                    } else {
                        self.op_error = Some(message.clone());
                        self.status_msg = format!("Error: {}", message);
                    }
                    if let Some(ref id) = dev_id {
                        if self.selected_id.as_deref() == Some(id) {
                            if let Some(dev) = self.devices.iter().find(|d| &d.id == id) {
                                let _ = self.worker_tx.send(WorkerMsg::GetDrivers {
                                    dev_id: id.clone(),
                                    path: dev.path.clone(),
                                });
                            }
                        }
                    }
                }
                BgResult::UdevTriggered => {
                    self.udev_event_count += 1;
                    if !self.op_busy {
                        let _ = self.worker_tx.send(WorkerMsg::Refresh);
                        self.op_busy = true;
                        self.status_msg = "Hardware change detected — rescanning…".to_string();
                    }
                }
                BgResult::AudioDevices { sinks, sources } => {
                    self.audio_sinks = sinks;
                    self.audio_sources = sources;
                    self.audio_busy = false;
                }
                BgResult::AudioResult { success, message } => {
                    self.audio_busy = false;
                    if success {
                        self.audio_error = None;
                        self.status_msg = message;
                    } else {
                        self.audio_error = Some(message);
                    }
                }
            }
        }

        // Expire old highlights
        self.recent_adds.retain(|_, t| t.elapsed() < HIGHLIGHT_DURATION);

        if !self.recent_adds.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        if needs_repaint {
            ctx.request_repaint();
        }
    }

    fn repopulate_prop_edits(&mut self, dev_id: &str) {
        if let Some(dev) = self.devices.iter().find(|d| d.id == dev_id) {
            for key in &dev.editable_properties {
                if !self.prop_edits.contains_key(key) {
                    let val = dev.properties.get(key).cloned().unwrap_or_default();
                    self.prop_edits.insert(key.clone(), val);
                }
            }
        }
    }

    // ── Selection ──────────────────────────────────────────────────────────────

    fn change_selection(&mut self, new_id: Option<String>) {
        if self.selected_id == new_id {
            return;
        }
        self.selected_id = new_id.clone();
        self.prop_edits.clear();
        self.available_drivers.clear();
        self.selected_driver.clear();
        self.driver_custom.clear();
        self.loading_drivers = false;
        self.op_error = None;

        if let Some(ref id) = new_id {
            self.repopulate_prop_edits(id);
            if let Some(dev) = self.devices.iter().find(|d| &d.id == id) {
                let _ = self.worker_tx.send(WorkerMsg::GetDrivers {
                    dev_id: id.clone(),
                    path: dev.path.clone(),
                });
                self.loading_drivers = true;
            }
        }
    }

    // ── Filtering & grouping ───────────────────────────────────────────────────

    fn is_relevant(dev: &Device) -> bool {
        match &dev.bus {
            Bus::Tty | Bus::IoMMU => false,
            Bus::Unknown(_) => false,
            _ => !dev.id.starts_with("dev-"),
        }
    }

    /// Filtered + grouped list (owned data avoids borrow issues in closures).
    fn filtered_groups(&self) -> Vec<(String, Vec<Device>)> {
        let q = self.search_query.to_lowercase();
        let bus_filter = self.bus_filter.to_lowercase();

        let filtered: Vec<Device> = self
            .devices
            .iter()
            .filter(|d| {
                // Show-all toggle
                if !self.show_all_devices && !Self::is_relevant(d) {
                    return false;
                }
                // Bus filter
                if !bus_filter.is_empty()
                    && d.bus.to_string().to_lowercase() != bus_filter
                {
                    return false;
                }
                // Text search
                if !q.is_empty() {
                    return d.name.to_lowercase().contains(&q)
                        || d.id.to_lowercase().contains(&q)
                        || d.vendor.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || d.model.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || d.driver.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || d.subsystem.to_lowercase().contains(&q);
                }
                true
            })
            .cloned()
            .collect();

        // Deterministic bus order
        let order: &[&str] = &[
            "USB", "PCIe", "Audio", "Input", "Block", "GPU/DRM", "Network",
            "HID", "Power", "IOMMU", "TTY",
        ];

        let mut map: HashMap<String, Vec<Device>> = HashMap::new();
        for dev in filtered {
            map.entry(dev.bus.to_string()).or_default().push(dev);
        }

        let mut result = Vec::new();
        for &bus in order {
            if let Some(mut devs) = map.remove(bus) {
                devs.sort_by(|a, b| a.name.cmp(&b.name));
                result.push((bus.to_string(), devs));
            }
        }
        let mut rest: Vec<_> = map.into_iter().collect();
        rest.sort_by_key(|(k, _)| k.clone());
        for (bus, mut devs) in rest {
            devs.sort_by(|a, b| a.name.cmp(&b.name));
            result.push((bus, devs));
        }
        result
    }

    // ── Color & icon helpers ───────────────────────────────────────────────────

    pub fn status_color(status: &DeviceStatus) -> Color32 {
        match status {
            DeviceStatus::Online => Color32::from_rgb(80, 200, 120),
            DeviceStatus::Suspended => Color32::from_rgb(230, 180, 40),
            DeviceStatus::Offline => Color32::from_rgb(200, 60, 60),
            DeviceStatus::Unbound => Color32::from_rgb(140, 140, 140),
            DeviceStatus::Error => Color32::from_rgb(230, 100, 30),
            DeviceStatus::Unknown => Color32::from_rgb(100, 100, 100),
        }
    }

    fn bus_icon(bus_label: &str) -> &'static str {
        match bus_label {
            "USB" => "🔌",
            "PCIe" => "🖥",
            "Audio" => "🔊",
            "Input" => "⌨",
            "Block" => "💾",
            "GPU/DRM" => "🎮",
            "Network" => "🌐",
            "HID" => "🖱",
            "TTY" => "📟",
            "Power" => "🔋",
            "IOMMU" => "◈",
            _ => "◦",
        }
    }

    fn prop_value_color(key: &str, val: &str) -> Color32 {
        let v = val.to_lowercase();
        if v.contains("error") || v.contains("fail") || (key == "authorized" && v == "0") {
            Color32::from_rgb(220, 80, 80)
        } else if v == "suspended" || v == "suspended-irq" {
            Color32::from_rgb(220, 175, 50)
        } else if v == "active" || v == "running" || (key == "authorized" && v == "1") {
            Color32::from_rgb(100, 220, 140)
        } else if v == "unsupported" || v == "unknown" {
            Color32::from_rgb(200, 140, 60)
        } else if v == "auto" || v == "enabled" || v == "on" {
            Color32::from_rgb(160, 200, 255)
        } else if v == "0" || v == "off" || v == "disabled" {
            Color32::from_rgb(130, 130, 130)
        } else {
            Color32::from_rgb(210, 210, 210)
        }
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for DmgrApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        self.poll_results(ctx);

        // Auto-refresh
        if !self.op_busy && self.last_scan.elapsed().as_secs() > AUTO_REFRESH_SECS {
            let _ = self.worker_tx.send(WorkerMsg::Refresh);
            self.op_busy = true;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // ── Confirm dialog ─────────────────────────────────────────────────────
        let mut confirmed_action: Option<WorkerMsg> = None;
        let mut cancel_confirm = false;

        if let Some(ref confirm) = self.confirm {
            let title = confirm.title.clone();
            let body = confirm.body.clone();
            let action = confirm.action.clone();

            egui::Window::new(&title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(460.0)
                .show(&ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&body).size(14.0));
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("  Confirm  ").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(200, 60, 40)),
                            )
                            .clicked()
                        {
                            confirmed_action = Some(action);
                        }
                        ui.add_space(8.0);
                        if ui.button("  Cancel  ").clicked() {
                            cancel_confirm = true;
                        }
                    });
                });
        }

        if cancel_confirm {
            self.confirm = None;
        }
        if let Some(action) = confirmed_action {
            self.confirm = None;
            let _ = self.worker_tx.send(action);
            self.op_busy = true;
            self.op_error = None;
        }

        // ── Audio window ───────────────────────────────────────────────────────
        if self.show_audio_window {
            let mut still_open = true;
            let mut switch_sink: Option<String> = None;
            let mut switch_source: Option<String> = None;

            egui::Window::new("🔊  Audio Devices")
                .open(&mut still_open)
                .resizable(true)
                .default_width(480.0)
                .default_height(400.0)
                .show(&ctx, |ui| {
                    if !self.audio_available {
                        ui.label(
                            RichText::new("⚠  pactl not found. Install pulseaudio or pipewire-pulse.")
                                .color(Color32::from_rgb(230, 150, 50)),
                        );
                        return;
                    }

                    // Refresh button
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.audio_busy, egui::Button::new("⟳ Refresh")).clicked() {
                            let _ = self.worker_tx.send(WorkerMsg::RefreshAudio);
                            self.audio_busy = true;
                        }
                        if self.audio_busy {
                            ui.spinner();
                        }
                        if let Some(ref err) = self.audio_error {
                            ui.label(
                                RichText::new(format!("⚠ {}", err))
                                    .color(Color32::from_rgb(220, 90, 60))
                                    .size(12.0),
                            );
                        }
                    });

                    ui.add_space(6.0);

                    // Output devices
                    ui.label(
                        RichText::new("Output Devices")
                            .strong()
                            .size(14.0)
                            .color(Color32::from_rgb(200, 200, 230)),
                    );
                    ui.separator();

                    if self.audio_sinks.is_empty() {
                        ui.label(
                            RichText::new("  No output devices found")
                                .color(Color32::from_rgb(120, 120, 120)),
                        );
                    }

                    let sinks = self.audio_sinks.clone();
                    for sink in &sinks {
                        let is_active = sink.is_default;
                        ui.horizontal(|ui| {
                            // Active indicator
                            if is_active {
                                ui.label(RichText::new("▶").color(Color32::from_rgb(80, 220, 120)).size(14.0));
                            } else {
                                ui.label(RichText::new("  ").size(14.0));
                            }

                            // Device icon
                            ui.label(RichText::new(sink.icon()).size(16.0));

                            // State dot
                            ui.label(
                                RichText::new("●").color(sink.state_color()).size(9.0),
                            );

                            // Description
                            let label_color = if is_active {
                                Color32::from_rgb(120, 230, 150)
                            } else {
                                Color32::from_rgb(210, 210, 210)
                            };
                            let resp = ui.selectable_label(
                                is_active,
                                RichText::new(&sink.description).color(label_color).size(13.0),
                            );
                            if resp.clicked() && !is_active {
                                switch_sink = Some(sink.name.clone());
                            }
                            if sink.muted {
                                ui.label(
                                    RichText::new("🔇 muted")
                                        .size(11.0)
                                        .color(Color32::from_rgb(180, 80, 80)),
                                );
                            }
                        });
                    }

                    ui.add_space(10.0);

                    // Input devices
                    ui.label(
                        RichText::new("Input Devices")
                            .strong()
                            .size(14.0)
                            .color(Color32::from_rgb(200, 200, 230)),
                    );
                    ui.separator();

                    if self.audio_sources.is_empty() {
                        ui.label(
                            RichText::new("  No input devices found")
                                .color(Color32::from_rgb(120, 120, 120)),
                        );
                    }

                    let sources = self.audio_sources.clone();
                    for source in &sources {
                        let is_active = source.is_default;
                        ui.horizontal(|ui| {
                            if is_active {
                                ui.label(
                                    RichText::new("▶").color(Color32::from_rgb(80, 220, 120)).size(14.0),
                                );
                            } else {
                                ui.label(RichText::new("  ").size(14.0));
                            }
                            ui.label(RichText::new("🎤").size(16.0));
                            ui.label(
                                RichText::new("●").color(source.state_color()).size(9.0),
                            );
                            let label_color = if is_active {
                                Color32::from_rgb(120, 230, 150)
                            } else {
                                Color32::from_rgb(210, 210, 210)
                            };
                            let resp = ui.selectable_label(
                                is_active,
                                RichText::new(&source.description).color(label_color).size(13.0),
                            );
                            if resp.clicked() && !is_active {
                                switch_source = Some(source.name.clone());
                            }
                            if source.muted {
                                ui.label(
                                    RichText::new("🔇 muted")
                                        .size(11.0)
                                        .color(Color32::from_rgb(180, 80, 80)),
                                );
                            }
                        });
                    }

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Click a device to switch to it")
                            .size(11.0)
                            .color(Color32::from_rgb(110, 110, 110))
                            .italics(),
                    );
                });

            if !still_open {
                self.show_audio_window = false;
            }
            if let Some(name) = switch_sink {
                let _ = self.worker_tx.send(WorkerMsg::SetDefaultSink(name));
                self.audio_busy = true;
            }
            if let Some(name) = switch_source {
                let _ = self.worker_tx.send(WorkerMsg::SetDefaultSource(name));
                self.audio_busy = true;
            }
        }

        // ── Toolbar ────────────────────────────────────────────────────────────
        egui::Panel::top("toolbar")
            .min_size(48.0)
            .show_inside(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    // Logo
                    ui.label(
                        RichText::new("⚙  dmgr")
                            .size(18.0)
                            .strong()
                            .color(Color32::from_rgb(180, 180, 255)),
                    );
                    ui.separator();

                    // Refresh
                    let refresh = ui.add_enabled(!self.op_busy, egui::Button::new("⟳  Refresh"));
                    if refresh.clicked() {
                        let _ = self.worker_tx.send(WorkerMsg::Refresh);
                        self.op_busy = true;
                        self.status_msg = "Scanning…".to_string();
                    }
                    if self.op_busy {
                        ui.spinner();
                    }
                    ui.separator();

                    // Audio button
                    let audio_color = if self.show_audio_window {
                        Color32::from_rgb(100, 200, 120)
                    } else {
                        Color32::from_rgb(180, 180, 180)
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("🔊  Audio").color(audio_color),
                            )
                            .fill(if self.show_audio_window {
                                Color32::from_rgb(40, 70, 50)
                            } else {
                                Color32::from_rgb(40, 40, 50)
                            }),
                        )
                        .on_hover_text("Switch audio output/input devices")
                        .clicked()
                    {
                        self.show_audio_window = !self.show_audio_window;
                        if self.show_audio_window && self.audio_sinks.is_empty() {
                            let _ = self.worker_tx.send(WorkerMsg::RefreshAudio);
                            self.audio_busy = true;
                        }
                    }
                    ui.separator();

                    // Search
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search by name, ID, driver…")
                            .desired_width(200.0),
                    );
                    if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                        self.search_query.clear();
                    }
                    ui.separator();

                    // Bus filter
                    ui.label("Bus:");
                    egui::ComboBox::from_id_salt("bus_filter_combo")
                        .selected_text(if self.bus_filter.is_empty() {
                            "All".to_string()
                        } else {
                            self.bus_filter.clone()
                        })
                        .width(110.0)
                        .show_ui(ui, |ui| {
                            for (label, val) in [
                                ("All buses", ""),
                                ("USB", "USB"),
                                ("PCIe", "PCIe"),
                                ("Audio", "Audio"),
                                ("Input", "Input"),
                                ("Block", "Block"),
                                ("GPU/DRM", "GPU/DRM"),
                                ("Network", "Network"),
                                ("HID", "HID"),
                                ("TTY", "TTY"),
                                ("Power", "Power"),
                            ] {
                                ui.selectable_value(
                                    &mut self.bus_filter,
                                    val.to_string(),
                                    label,
                                );
                            }
                        });
                    ui.separator();

                    // Show all toggle
                    let show_all_label = if self.show_all_devices { "👁 All" } else { "👁 Filtered" };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(show_all_label).size(12.0),
                            )
                            .fill(if self.show_all_devices {
                                Color32::from_rgb(60, 70, 90)
                            } else {
                                Color32::from_rgb(35, 35, 45)
                            }),
                        )
                        .on_hover_text("Toggle: show TTY/noise devices")
                        .clicked()
                    {
                        self.show_all_devices = !self.show_all_devices;
                    }
                });
                ui.add_space(4.0);
            });

        // ── Status bar ─────────────────────────────────────────────────────────
        egui::Panel::bottom("status_bar")
            .min_size(24.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("{} devices", self.scan_count))
                            .size(12.0)
                            .color(Color32::from_rgb(170, 170, 170)),
                    );
                    ui.separator();

                    let secs = self.last_scan.elapsed().as_secs();
                    let t = if secs < 60 { format!("{}s ago", secs) }
                            else { format!("{}m ago", secs / 60) };
                    ui.label(
                        RichText::new(format!("Last scan: {}", t))
                            .size(12.0)
                            .color(Color32::from_rgb(130, 130, 130)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("udev ● {}", self.udev_event_count))
                            .size(12.0)
                            .color(Color32::from_rgb(80, 200, 120)),
                    );
                    ui.separator();

                    if let Some(ref err) = self.op_error {
                        ui.label(
                            RichText::new(format!("⚠ {}", err))
                                .size(12.0)
                                .color(Color32::from_rgb(230, 100, 60)),
                        );
                    } else {
                        ui.label(
                            RichText::new(&self.status_msg)
                                .size(12.0)
                                .color(Color32::from_rgb(160, 200, 160)),
                        );
                    }
                });
            });

        // ── Device list (left panel) ───────────────────────────────────────────
        let mut new_selection: Option<Option<String>> = None;
        let groups = self.filtered_groups();
        let op_busy = self.op_busy;
        let selected_id = self.selected_id.clone();
        let recent_adds = self.recent_adds.clone();

        egui::Panel::left("device_list")
            .default_size(300.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Devices")
                            .strong()
                            .size(13.0)
                            .color(Color32::from_rgb(200, 200, 220)),
                    );
                });
                ui.separator();

                ScrollArea::vertical()
                    .id_salt("device_list_scroll")
                    .show(ui, |ui| {
                        if groups.is_empty() {
                            ui.add_space(40.0);
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new(if op_busy {
                                        "⟳  Scanning…"
                                    } else {
                                        "No devices match the current filter"
                                    })
                                    .size(13.0)
                                    .color(Color32::from_rgb(110, 110, 120)),
                                );
                            });
                            return;
                        }

                        for (bus_label, bus_devices) in &groups {
                            let icon = DmgrApp::bus_icon(bus_label);
                            let count = bus_devices.len();
                            let header = RichText::new(format!(
                                "{}  {} ",
                                icon, bus_label,
                            ))
                            .strong()
                            .size(13.0);

                            egui::CollapsingHeader::new(header)
                                .default_open(true)
                                .show(ui, |ui| {
                                    // Count badge
                                    ui.horizontal(|ui| {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(format!("{} devices", count))
                                                .size(10.5)
                                                .color(Color32::from_rgb(100, 100, 120))
                                                .italics(),
                                        );
                                    });

                                    for device in bus_devices {
                                        let selected =
                                            selected_id.as_deref() == Some(&device.id);

                                        // Hotplug highlight: compute fade
                                        let add_progress = recent_adds.get(&device.id).map(|t| {
                                            1.0 - (t.elapsed().as_secs_f32()
                                                / HIGHLIGHT_DURATION.as_secs_f32())
                                            .clamp(0.0, 1.0)
                                        });

                                        let label_color = if let Some(p) = add_progress {
                                            // Fade from green to normal
                                            let g = (80.0 + 140.0 * p) as u8;
                                            let r = (180.0 * (1.0 - p)) as u8;
                                            Color32::from_rgb(r, g, 100)
                                        } else {
                                            ui.visuals().text_color()
                                        };

                                        let display_name = if device.name.len() > 36 {
                                            format!("{}…", &device.name[..35])
                                        } else {
                                            device.name.clone()
                                        };

                                        let resp = ui.horizontal(|ui| {
                                            ui.add_space(2.0);
                                            let sc = DmgrApp::status_color(&device.status);
                                            ui.label(RichText::new("●").color(sc).size(9.0));

                                            ui.selectable_label(
                                                selected,
                                                RichText::new(&display_name)
                                                    .size(12.5)
                                                    .color(label_color),
                                            )
                                        });

                                        if resp.inner.clicked() {
                                            new_selection = Some(Some(device.id.clone()));
                                        }

                                        // Tooltip with full info
                                        resp.response.on_hover_ui(|ui| {
                                            ui.label(
                                                RichText::new(&device.name).strong().size(12.0),
                                            );
                                            ui.label(
                                                RichText::new(format!("ID: {}", device.id))
                                                    .monospace()
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(150, 150, 150)),
                                            );
                                            if let Some(ref drv) = device.driver {
                                                ui.label(
                                                    RichText::new(format!("Driver: {}", drv))
                                                        .size(11.0)
                                                        .color(Color32::from_rgb(160, 220, 160)),
                                                );
                                            }
                                            ui.label(
                                                RichText::new(format!(
                                                    "Status: {}",
                                                    device.status
                                                ))
                                                .size(11.0)
                                                .color(DmgrApp::status_color(&device.status)),
                                            );
                                        });
                                    }
                                    ui.add_space(2.0);
                                });
                        }

                        ui.add_space(8.0);
                    });
            });

        // ── Device detail (central panel) ──────────────────────────────────────
        let mut pending_confirm: Option<ConfirmPending> = None;

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let device: Option<Device> = self
                .selected_id
                .as_ref()
                .and_then(|id| self.devices.iter().find(|d| &d.id == id))
                .cloned();

            match device {
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(if op_busy {
                                "⟳  Scanning devices…"
                            } else {
                                "← Select a device from the list"
                            })
                            .size(16.0)
                            .color(Color32::from_rgb(90, 90, 110)),
                        );
                    });
                }
                Some(dev) => {
                    ScrollArea::vertical()
                        .id_salt("detail_scroll")
                        .show(ui, |ui| {
                            render_detail(
                                ui,
                                &dev,
                                &self.available_drivers,
                                self.loading_drivers,
                                &mut self.prop_edits,
                                &mut self.selected_driver,
                                &mut self.driver_custom,
                                &mut pending_confirm,
                            );
                        });
                }
            }
        });

        // ── Apply deferred state ───────────────────────────────────────────────
        if let Some(sel) = new_selection {
            self.change_selection(sel);
        }
        if let Some(confirm) = pending_confirm {
            self.confirm = Some(confirm);
        }
    }
}

// ── Device Detail ─────────────────────────────────────────────────────────────

fn render_detail(
    ui: &mut egui::Ui,
    dev: &Device,
    available_drivers: &[String],
    loading_drivers: bool,
    prop_edits: &mut HashMap<String, String>,
    sel_driver: &mut String,
    drv_custom: &mut String,
    pending_confirm: &mut Option<ConfirmPending>,
) {
    // ── Header ────────────────────────────────────────────────────────────────
    let sc = DmgrApp::status_color(&dev.status);

    ui.horizontal(|ui| {
        ui.label(RichText::new("●").color(sc).size(24.0));
        ui.add_space(4.0);
        ui.label(RichText::new(&dev.name).strong().size(20.0));
    });

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(dev.bus.to_string())
                .strong()
                .size(13.0)
                .color(Color32::from_rgb(180, 180, 255)),
        );
        ui.separator();
        ui.label(RichText::new(dev.status.to_string()).color(sc).size(13.0));
        if dev.removable {
            ui.separator();
            ui.label(
                RichText::new("removable")
                    .size(12.0)
                    .color(Color32::from_rgb(180, 210, 240)),
            );
        }
        if !dev.authorized {
            ui.separator();
            ui.label(
                RichText::new("⚠ unauthorized")
                    .size(12.0)
                    .color(Color32::from_rgb(230, 150, 50)),
            );
        }
    });

    ui.add_space(6.0);
    ui.separator();

    // ── Identity ──────────────────────────────────────────────────────────────
    section_header(ui, "🔎  Identity");

    egui::Grid::new("identity_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            kv(ui, "ID", &dev.id);
            kv(ui, "Subsystem", &dev.subsystem);
            if let Some(ref v) = dev.bus_id {
                kv(ui, "Bus ID", v);
            }
            if let Some(ref vendor) = dev.vendor {
                let label = match &dev.vendor_id {
                    Some(id) => format!("{} [{}]", vendor, id),
                    None => vendor.clone(),
                };
                kv(ui, "Vendor", &label);
            } else if let Some(ref vid) = dev.vendor_id {
                kv(ui, "Vendor ID", vid);
            }
            if let Some(ref model) = dev.model {
                let label = match &dev.model_id {
                    Some(id) => format!("{} [{}]", model, id),
                    None => model.clone(),
                };
                kv(ui, "Model", &label);
            } else if let Some(ref mid) = dev.model_id {
                kv(ui, "Model ID", mid);
            }
            kv(ui, "sysfs path", &dev.path);
        });

    ui.add_space(8.0);

    // ── Driver section ────────────────────────────────────────────────────────
    section_header(ui, "🔧  Driver");

    if let Some(ref driver) = dev.driver {
        // Device has a driver bound
        ui.horizontal(|ui| {
            ui.label(RichText::new("Current: ").size(13.0));
            ui.label(
                RichText::new(driver)
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(160, 220, 160)),
            );
            ui.add_space(8.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("  Unbind Driver  ").color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(190, 55, 40)),
                )
                .on_hover_text("Remove this driver (requires root). Device may stop working.")
                .clicked()
            {
                *pending_confirm = Some(ConfirmPending {
                    title: "Unbind Driver".to_string(),
                    body: format!(
                        "Unbind driver '{}' from '{}' ?\n\nThe device may stop functioning.",
                        driver, dev.name
                    ),
                    action: WorkerMsg::UnbindDriver {
                        dev_id: dev.id.clone(),
                        path: dev.path.clone(),
                    },
                });
            }
        });
    } else {
        // No driver — show bind section
        ui.label(
            RichText::new("No driver bound")
                .color(Color32::from_rgb(150, 150, 150))
                .size(13.0),
        );
        ui.add_space(6.0);

        ui.group(|ui| {
            ui.label(RichText::new("Bind Driver").strong().size(13.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("From list:");
                if loading_drivers {
                    ui.spinner();
                    ui.label(RichText::new("Loading…").color(Color32::from_rgb(140, 140, 140)));
                } else if available_drivers.is_empty() {
                    ui.label(
                        RichText::new("No compatible drivers found")
                            .color(Color32::from_rgb(120, 120, 120)),
                    );
                } else {
                    egui::ComboBox::from_id_salt("driver_combo")
                        .selected_text(if sel_driver.is_empty() {
                            "Select…".to_string()
                        } else {
                            sel_driver.clone()
                        })
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            for d in available_drivers {
                                ui.selectable_value(sel_driver, d.clone(), d);
                            }
                        });
                }
            });

            ui.horizontal(|ui| {
                ui.label("Or type:  ");
                ui.add(
                    egui::TextEdit::singleline(drv_custom)
                        .hint_text("driver_name")
                        .desired_width(220.0),
                );
            });

            let driver_to_bind = if drv_custom.trim().is_empty() {
                sel_driver.clone()
            } else {
                drv_custom.trim().to_string()
            };

            ui.add_space(4.0);
            let can_bind = !driver_to_bind.is_empty();

            if ui
                .add_enabled(
                    can_bind,
                    egui::Button::new(
                        RichText::new(if can_bind {
                            format!("  Bind '{}'  ", driver_to_bind)
                        } else {
                            "  Bind Driver  ".to_string()
                        })
                        .color(Color32::WHITE),
                    )
                    .fill(if can_bind {
                        Color32::from_rgb(50, 150, 80)
                    } else {
                        Color32::from_rgb(50, 60, 50)
                    }),
                )
                .on_hover_text("Bind the selected driver to this device (requires root)")
                .clicked()
            {
                *pending_confirm = Some(ConfirmPending {
                    title: "Bind Driver".to_string(),
                    body: format!(
                        "Bind driver '{}' to '{}' ?\n\nThis may affect system hardware.",
                        driver_to_bind, dev.name
                    ),
                    action: WorkerMsg::BindDriver {
                        dev_id: dev.id.clone(),
                        path: dev.path.clone(),
                        driver: driver_to_bind,
                    },
                });
            }
        });
    }

    ui.add_space(8.0);

    // ── Editable Properties ───────────────────────────────────────────────────
    if !dev.editable_properties.is_empty() {
        section_header(ui, "✏  Editable Properties");

        let mut reset_attr: Option<String> = None;
        let mut set_confirm: Option<ConfirmPending> = None;

        for attr in &dev.editable_properties {
            let current = dev.properties.get(attr).cloned().unwrap_or_default();
            prop_edits.entry(attr.clone()).or_insert_with(|| current.clone());
            let changed = prop_edits.get(attr).map(|v| v != &current).unwrap_or(false);

            let mut clicked_set = false;
            let mut clicked_reset = false;
            {
                let edit_val = prop_edits.get_mut(attr).unwrap();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:<26}", attr))
                            .monospace()
                            .size(12.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(edit_val)
                            .desired_width(140.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    let btn_color = if changed {
                        Color32::from_rgb(50, 120, 200)
                    } else {
                        Color32::from_rgb(50, 55, 70)
                    };
                    if ui
                        .add_enabled(
                            changed,
                            egui::Button::new(RichText::new("Set").color(Color32::WHITE))
                                .fill(btn_color),
                        )
                        .on_hover_text("Write this value to sysfs (may require root)")
                        .clicked()
                    {
                        clicked_set = true;
                    }
                    if changed
                        && ui
                            .small_button("↩")
                            .on_hover_text("Reset to current value")
                            .clicked()
                    {
                        clicked_reset = true;
                    }
                });
            }

            if clicked_set {
                let new_val = prop_edits.get(attr).cloned().unwrap_or_default();
                set_confirm = Some(ConfirmPending {
                    title: "Set Property".to_string(),
                    body: format!("Set '{}' = '{}' on '{}' ?", attr, new_val, dev.name),
                    action: WorkerMsg::SetProperty {
                        dev_id: dev.id.clone(),
                        path: dev.path.clone(),
                        attr: attr.clone(),
                        value: new_val,
                    },
                });
            }
            if clicked_reset {
                reset_attr = Some(attr.clone());
            }
        }

        if let Some(ref attr) = reset_attr {
            let current = dev.properties.get(attr).cloned().unwrap_or_default();
            prop_edits.insert(attr.clone(), current);
        }
        if set_confirm.is_some() && pending_confirm.is_none() {
            *pending_confirm = set_confirm;
        }

        ui.add_space(8.0);
    }

    // ── All Properties (grouped + color-coded) ────────────────────────────────
    if !dev.properties.is_empty() {
        // Group properties by prefix
        let mut power: Vec<(&String, &String)> = Vec::new();
        let mut access: Vec<(&String, &String)> = Vec::new();
        let mut general: Vec<(&String, &String)> = Vec::new();

        let mut sorted_props: Vec<_> = dev.properties.iter().collect();
        sorted_props.sort_by_key(|(k, _)| k.as_str());

        for (k, v) in &sorted_props {
            if k.starts_with("power/") {
                power.push((k, v));
            } else if matches!(k.as_str(), "authorized" | "removable" | "iommu_group" | "numa_node") {
                access.push((k, v));
            } else {
                general.push((k, v));
            }
        }

        if !power.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new("⚡  Power Management")
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(200, 200, 230)),
            )
            .default_open(true)
            .show(ui, |ui| {
                prop_grid(ui, "power_grid", &power, &dev.editable_properties);
            });
            ui.add_space(4.0);
        }

        if !access.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new("🔐  Access Control")
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(200, 200, 230)),
            )
            .default_open(true)
            .show(ui, |ui| {
                prop_grid(ui, "access_grid", &access, &dev.editable_properties);
            });
            ui.add_space(4.0);
        }

        if !general.is_empty() {
            egui::CollapsingHeader::new(
                RichText::new("📋  sysfs Properties")
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(200, 200, 230)),
            )
            .default_open(false)
            .show(ui, |ui| {
                prop_grid(ui, "general_grid", &general, &dev.editable_properties);
            });
            ui.add_space(4.0);
        }
    }

    // ── Interfaces ────────────────────────────────────────────────────────────
    if !dev.interfaces.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new("🔌  Interfaces")
                .strong()
                .size(13.0)
                .color(Color32::from_rgb(200, 200, 230)),
        )
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("ifaces_grid")
                .num_columns(3)
                .spacing([12.0, 3.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("Name").strong().size(12.0));
                    ui.label(RichText::new("Class").strong().size(12.0));
                    ui.label(RichText::new("Protocol").strong().size(12.0));
                    ui.end_row();
                    for iface in &dev.interfaces {
                        ui.label(RichText::new(&iface.name).monospace().size(11.5));
                        ui.label(RichText::new(&iface.class).monospace().size(11.5));
                        ui.label(
                            RichText::new(iface.protocol.as_deref().unwrap_or("—"))
                                .monospace()
                                .size(11.5),
                        );
                        ui.end_row();
                    }
                });
        });
        ui.add_space(4.0);
    }

    // ── Hierarchy ─────────────────────────────────────────────────────────────
    if dev.parent.is_some() || !dev.children.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new("🌲  Hierarchy")
                .strong()
                .size(13.0)
                .color(Color32::from_rgb(200, 200, 230)),
        )
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("hierarchy_grid")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    if let Some(ref parent) = dev.parent {
                        kv(ui, "Parent", parent);
                    }
                    if !dev.children.is_empty() {
                        ui.label(
                            RichText::new("Children")
                                .size(12.0)
                                .color(Color32::from_rgb(160, 160, 180)),
                        );
                        ui.vertical(|ui| {
                            for child in &dev.children {
                                ui.label(RichText::new(child).monospace().size(11.0));
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    }

    ui.add_space(24.0);
}

// ── UI helpers ────────────────────────────────────────────────────────────────

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .strong()
                .size(13.5)
                .color(Color32::from_rgb(200, 200, 230)),
        );
        ui.separator();
    });
    ui.add_space(4.0);
}

fn kv(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.label(
        RichText::new(key)
            .size(12.0)
            .color(Color32::from_rgb(155, 155, 175)),
    );
    ui.label(RichText::new(value).monospace().size(12.0));
    ui.end_row();
}

fn prop_grid(
    ui: &mut egui::Ui,
    id: &str,
    props: &[(&String, &String)],
    editable: &[String],
) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 2.0])
        .striped(true)
        .show(ui, |ui| {
            for (key, val) in props {
                let is_editable = editable.contains(key);
                let key_color = if is_editable {
                    Color32::from_rgb(130, 190, 255)
                } else {
                    Color32::from_rgb(145, 145, 155)
                };
                ui.label(
                    RichText::new(key.as_str())
                        .monospace()
                        .size(11.5)
                        .color(key_color),
                );
                let val_color = DmgrApp::prop_value_color(key, val);
                ui.label(
                    RichText::new(val.as_str())
                        .monospace()
                        .size(11.5)
                        .color(val_color),
                );
                ui.end_row();
            }
        });
}
