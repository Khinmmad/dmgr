use crate::worker::{BgResult, WorkerMsg};
use dmgr_core::device::{Bus, Device, DeviceStatus};
use egui::{Color32, RichText, ScrollArea};
use std::collections::HashMap;
use std::sync::mpsc;

// ── Confirm dialog state ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ConfirmPending {
    title: String,
    body: String,
    action: WorkerMsg,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct DmgrApp {
    // Device data
    devices: Vec<Device>,
    selected_id: Option<String>,

    // Filters
    search_query: String,
    bus_filter: String,

    // Driver controls (for selected device)
    available_drivers: Vec<String>,
    selected_driver: String,
    driver_custom: String,
    loading_drivers: bool,

    // Property editing: attribute → edited value
    prop_edits: HashMap<String, String>,

    // Background channels
    worker_tx: mpsc::Sender<WorkerMsg>,
    bg_rx: mpsc::Receiver<BgResult>,

    // Feedback
    status_msg: String,
    op_error: Option<String>,
    op_busy: bool,

    // Confirm dialog
    confirm: Option<ConfirmPending>,

    // Stats
    udev_event_count: u32,
    last_scan: std::time::Instant,
    scan_count: usize,

    // Collapse state per bus group
    bus_collapsed: HashMap<String, bool>,
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
            udev_event_count: 0,
            last_scan: std::time::Instant::now(),
            scan_count: 0,
            bus_collapsed: HashMap::new(),
        }
    }

    // ── Background polling ────────────────────────────────────────────────────

    fn poll_results(&mut self, ctx: &egui::Context) {
        let mut needs_repaint = false;

        while let Ok(result) = self.bg_rx.try_recv() {
            needs_repaint = true;
            match result {
                BgResult::Scanned(devices) => {
                    self.scan_count = devices.len();
                    self.devices = devices;
                    self.last_scan = std::time::Instant::now();
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
                    // Reload drivers for the affected device
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
            }
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

    // ── Selection ─────────────────────────────────────────────────────────────

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

    // ── Filtering / grouping ──────────────────────────────────────────────────

    /// Returns cloned + filtered + grouped device list (owned data avoids borrow issues).
    fn filtered_groups(&self) -> Vec<(String, Vec<Device>)> {
        let q = self.search_query.to_lowercase();
        let bus_filter = self.bus_filter.to_lowercase();

        let filtered: Vec<Device> = self
            .devices
            .iter()
            .filter(|d| {
                if !bus_filter.is_empty()
                    && d.bus.to_string().to_lowercase() != bus_filter
                {
                    return false;
                }
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

        // Group by bus in a deterministic order
        let order: &[&str] = &[
            "USB", "PCIe", "Audio", "Input", "Block", "GPU/DRM", "Network",
            "HID", "TTY", "Power", "IOMMU",
        ];

        let mut map: HashMap<String, Vec<Device>> = HashMap::new();
        for dev in filtered {
            map.entry(dev.bus.to_string()).or_default().push(dev);
        }

        let mut result = Vec::new();
        for &bus in order {
            if let Some(devs) = map.remove(bus) {
                result.push((bus.to_string(), devs));
            }
        }
        let mut rest: Vec<_> = map.into_iter().collect();
        rest.sort_by_key(|(k, _)| k.clone());
        result.extend(rest);
        result
    }

    // ── Color helpers ─────────────────────────────────────────────────────────

    pub fn status_color(status: &DeviceStatus) -> Color32 {
        match status {
            DeviceStatus::Online => Color32::from_rgb(80, 200, 120),
            DeviceStatus::Suspended => Color32::from_rgb(230, 180, 40),
            DeviceStatus::Offline => Color32::from_rgb(200, 60, 60),
            DeviceStatus::Unbound => Color32::from_rgb(160, 160, 160),
            DeviceStatus::Error => Color32::from_rgb(230, 100, 30),
            DeviceStatus::Unknown => Color32::from_rgb(100, 100, 100),
        }
    }

    pub fn bus_color(bus: &Bus) -> Color32 {
        match bus {
            Bus::Usb => Color32::from_rgb(80, 150, 230),
            Bus::Pci => Color32::from_rgb(180, 100, 230),
            Bus::Audio => Color32::from_rgb(230, 150, 80),
            Bus::Input => Color32::from_rgb(80, 200, 180),
            Bus::Block => Color32::from_rgb(200, 160, 80),
            Bus::Drm => Color32::from_rgb(100, 200, 80),
            Bus::Net => Color32::from_rgb(80, 180, 230),
            Bus::Hid => Color32::from_rgb(200, 120, 120),
            Bus::Tty => Color32::from_rgb(160, 160, 160),
            Bus::Power => Color32::from_rgb(230, 230, 80),
            Bus::IoMMU => Color32::from_rgb(180, 180, 230),
            Bus::Unknown(_) => Color32::from_rgb(130, 130, 130),
        }
    }
}

// ── eframe::App (eframe 0.34 API) ─────────────────────────────────────────────

impl eframe::App for DmgrApp {
    /// Called before each UI frame — use for non-rendering logic.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        self.poll_results(ctx);
    }

    /// Main UI method — receives the root Ui.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // ── Confirm dialog ────────────────────────────────────────────────────
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
                .default_width(440.0)
                .show(&ctx, |ui| {
                    ui.label(RichText::new(&body).size(14.0));
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("  Confirm  ").color(Color32::BLACK),
                                )
                                .fill(Color32::from_rgb(220, 80, 60)),
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

        // ── Toolbar ───────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .min_size(44.0)
            .show_inside(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚙  dmgr")
                            .size(18.0)
                            .strong()
                            .color(Color32::from_rgb(180, 180, 255)),
                    );
                    ui.separator();

                    let refresh_btn = ui.add_enabled(
                        !self.op_busy,
                        egui::Button::new("⟳  Refresh"),
                    );
                    if refresh_btn.clicked() {
                        let _ = self.worker_tx.send(WorkerMsg::Refresh);
                        self.op_busy = true;
                        self.status_msg = "Scanning…".to_string();
                    }
                    if self.op_busy {
                        ui.spinner();
                    }

                    ui.separator();
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search devices…")
                            .desired_width(220.0),
                    );
                    if !self.search_query.is_empty() && ui.small_button("✕").clicked() {
                        self.search_query.clear();
                    }

                    ui.separator();
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
                                ("All", ""),
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
                });
                ui.add_space(4.0);
            });

        // ── Status bar ────────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .min_size(24.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let c = Color32::from_rgb(170, 170, 170);
                    ui.label(
                        RichText::new(format!("{} devices", self.scan_count))
                            .size(12.0)
                            .color(c),
                    );
                    ui.separator();

                    let secs = self.last_scan.elapsed().as_secs();
                    let t = if secs < 60 {
                        format!("{}s ago", secs)
                    } else if secs < 3600 {
                        format!("{}m ago", secs / 60)
                    } else {
                        format!("{}h ago", secs / 3600)
                    };
                    ui.label(
                        RichText::new(format!("Last scan: {}", t))
                            .size(12.0)
                            .color(Color32::from_rgb(140, 140, 140)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!("udev ● ({})", self.udev_event_count))
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

        // ── Left panel: device list ───────────────────────────────────────────
        let mut new_selection: Option<Option<String>> = None;
        // Pre-compute groups (owned data) to avoid borrow issues inside closure
        let groups = self.filtered_groups();
        let op_busy = self.op_busy;

        egui::SidePanel::left("device_list")
            .default_size(280.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Devices")
                        .strong()
                        .size(13.0)
                        .color(Color32::from_rgb(200, 200, 220)),
                );
                ui.separator();

                ScrollArea::vertical()
                    .id_salt("device_list_scroll")
                    .show(ui, |ui| {
                        if groups.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new(if op_busy {
                                        "Scanning…"
                                    } else {
                                        "No devices found"
                                    })
                                    .color(Color32::from_rgb(120, 120, 120)),
                                );
                            });
                            return;
                        }

                        for (bus_label, bus_devices) in &groups {
                            let header = format!("{} ({})", bus_label, bus_devices.len());

                            egui::CollapsingHeader::new(
                                RichText::new(&header).strong().size(12.5),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                for device in bus_devices {
                                    let selected = self.selected_id.as_deref()
                                        == Some(&device.id);

                                    ui.horizontal(|ui| {
                                        let color = Self::status_color(&device.status);
                                        ui.label(RichText::new("●").color(color).size(9.0));

                                        let display_name = if device.name.len() > 34 {
                                            format!("{}…", &device.name[..33])
                                        } else {
                                            device.name.clone()
                                        };

                                        if ui
                                            .selectable_label(
                                                selected,
                                                RichText::new(&display_name).size(12.5),
                                            )
                                            .clicked()
                                        {
                                            new_selection = Some(Some(device.id.clone()));
                                        }
                                    });
                                }
                            });
                        }
                    });
            });

        // ── Central panel: device detail ──────────────────────────────────────
        let mut pending_action: Option<WorkerMsg> = None;
        let mut pending_confirm: Option<ConfirmPending> = None;

        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Clone selected device to avoid borrow conflicts
            let device: Option<Device> = self
                .selected_id
                .as_ref()
                .and_then(|id| self.devices.iter().find(|d| &d.id == id))
                .cloned();

            match device {
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(if self.op_busy {
                                "⟳  Scanning devices…"
                            } else {
                                "← Select a device from the list"
                            })
                            .size(16.0)
                            .color(Color32::from_rgb(100, 100, 120)),
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
                                &mut pending_action,
                                &mut pending_confirm,
                            );
                        });
                }
            }
        });

        // ── Apply queued state changes ─────────────────────────────────────────
        if let Some(sel) = new_selection {
            self.change_selection(sel);
        }
        if let Some(action) = pending_action {
            let _ = self.worker_tx.send(action);
            self.op_busy = true;
            self.op_error = None;
        }
        if let Some(confirm) = pending_confirm {
            self.confirm = Some(confirm);
        }
    }
}

// ── Device detail renderer ────────────────────────────────────────────────────

fn render_detail(
    ui: &mut egui::Ui,
    dev: &Device,
    available_drivers: &[String],
    loading_drivers: bool,
    prop_edits: &mut HashMap<String, String>,
    sel_driver: &mut String,
    drv_custom: &mut String,
    _pending_action: &mut Option<WorkerMsg>,
    pending_confirm: &mut Option<ConfirmPending>,
) {
    // ── Header ────────────────────────────────────────────────────────────────
    let status_color = DmgrApp::status_color(&dev.status);
    let bus_color = DmgrApp::bus_color(&dev.bus);

    ui.horizontal(|ui| {
        ui.label(RichText::new("●").color(status_color).size(22.0));
        ui.add_space(4.0);
        ui.label(RichText::new(&dev.name).strong().size(20.0));
    });

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(dev.bus.to_string())
                .color(bus_color)
                .size(13.0)
                .strong(),
        );
        ui.separator();
        ui.label(RichText::new(dev.status.to_string()).color(status_color).size(13.0));
        if dev.removable {
            ui.separator();
            ui.label(
                RichText::new("removable")
                    .size(12.0)
                    .color(Color32::from_rgb(180, 200, 230)),
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
    section_header(ui, "Identity");
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
                    Some(id) => format!("{} ({})", vendor, id),
                    None => vendor.clone(),
                };
                kv(ui, "Vendor", &label);
            } else if let Some(ref vid) = dev.vendor_id {
                kv(ui, "Vendor ID", vid);
            }
            if let Some(ref model) = dev.model {
                let label = match &dev.model_id {
                    Some(id) => format!("{} ({})", model, id),
                    None => model.clone(),
                };
                kv(ui, "Model", &label);
            } else if let Some(ref mid) = dev.model_id {
                kv(ui, "Model ID", mid);
            }
            kv(ui, "sysfs path", &dev.path);
        });

    ui.add_space(8.0);

    // ── Driver management ─────────────────────────────────────────────────────
    section_header(ui, "Driver");

    // Current driver + unbind
    ui.horizontal(|ui| {
        ui.label("Current:");
        match &dev.driver {
            Some(driver) => {
                ui.label(
                    RichText::new(driver)
                        .strong()
                        .color(Color32::from_rgb(160, 220, 160)),
                );
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Unbind Driver").color(Color32::BLACK),
                        )
                        .fill(Color32::from_rgb(210, 70, 50)),
                    )
                    .on_hover_text("Remove this driver binding (requires root)")
                    .clicked()
                {
                    *pending_confirm = Some(ConfirmPending {
                        title: "Unbind Driver".to_string(),
                        body: format!(
                            "Unbind driver '{}' from '{}' ?\n\nThe device may stop working.",
                            driver, dev.name
                        ),
                        action: WorkerMsg::UnbindDriver {
                            dev_id: dev.id.clone(),
                            path: dev.path.clone(),
                        },
                    });
                }
            }
            None => {
                ui.label(
                    RichText::new("None").color(Color32::from_rgb(150, 150, 150)),
                );
            }
        }
    });

    ui.add_space(6.0);

    // Bind new driver
    ui.group(|ui| {
        ui.label(RichText::new("Bind Driver").strong().size(13.0));
        ui.add_space(4.0);

        // From available list
        ui.horizontal(|ui| {
            ui.label("From list: ");
            if loading_drivers {
                ui.spinner();
                ui.label(
                    RichText::new("Loading…").color(Color32::from_rgb(150, 150, 150)),
                );
            } else if available_drivers.is_empty() {
                ui.label(
                    RichText::new("No compatible drivers found")
                        .color(Color32::from_rgb(130, 130, 130)),
                );
            } else {
                egui::ComboBox::from_id_salt("driver_combo")
                    .selected_text(if sel_driver.is_empty() {
                        "Select driver…".to_string()
                    } else {
                        sel_driver.clone()
                    })
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for driver in available_drivers {
                            ui.selectable_value(sel_driver, driver.clone(), driver);
                        }
                    });
            }
        });

        // Custom override
        ui.horizontal(|ui| {
            ui.label("Or type:   ");
            ui.add(
                egui::TextEdit::singleline(drv_custom)
                    .hint_text("driver_name (overrides list)")
                    .desired_width(200.0),
            );
        });

        ui.add_space(4.0);

        // Which driver to use
        let driver_to_bind = if drv_custom.trim().is_empty() {
            sel_driver.clone()
        } else {
            drv_custom.trim().to_string()
        };

        let can_bind = !driver_to_bind.is_empty();
        let btn_label = if driver_to_bind.is_empty() {
            "Bind Driver".to_string()
        } else {
            format!("  Bind '{}'  ", driver_to_bind)
        };

        if ui
            .add_enabled(
                can_bind,
                egui::Button::new(
                    RichText::new(btn_label).color(Color32::BLACK),
                )
                .fill(if can_bind {
                    Color32::from_rgb(60, 160, 90)
                } else {
                    Color32::from_rgb(60, 80, 60)
                }),
            )
            .on_hover_text("Bind the selected driver (requires root)")
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

    ui.add_space(8.0);

    // ── Editable Properties ───────────────────────────────────────────────────
    if !dev.editable_properties.is_empty() {
        section_header(ui, "Editable Properties");

        // Collect deferred mutations to avoid borrow conflicts inside closures
        let mut reset_attr: Option<String> = None;
        let mut set_confirm: Option<ConfirmPending> = None;

        for attr in &dev.editable_properties {
            let current = dev.properties.get(attr).cloned().unwrap_or_default();
            // Ensure entry exists
            prop_edits.entry(attr.clone()).or_insert_with(|| current.clone());

            let changed = prop_edits.get(attr).map(|v| v != &current).unwrap_or(false);

            // Borrow prop_edits mutably for TextEdit, then drop before reset
            let mut clicked_set = false;
            let mut clicked_reset = false;
            {
                let edit_val = prop_edits.get_mut(attr).unwrap();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:<26}", attr))
                            .monospace()
                            .size(12.5),
                    );
                    ui.add(
                        egui::TextEdit::singleline(edit_val)
                            .desired_width(150.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    if ui
                        .add_enabled(
                            changed,
                            egui::Button::new(RichText::new("Set").color(Color32::BLACK))
                                .fill(if changed {
                                    Color32::from_rgb(60, 130, 200)
                                } else {
                                    Color32::from_rgb(60, 60, 80)
                                }),
                        )
                        .on_hover_text("Write this value to sysfs (may require root)")
                        .clicked()
                    {
                        clicked_set = true;
                    }
                    if changed
                        && ui
                            .small_button("↩")
                            .on_hover_text("Reset to current sysfs value")
                            .clicked()
                    {
                        clicked_reset = true;
                    }
                });
            } // edit_val (mutable borrow) dropped here

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

        // Apply deferred mutations
        if let Some(ref attr) = reset_attr {
            let current = dev.properties.get(attr).cloned().unwrap_or_default();
            prop_edits.insert(attr.clone(), current);
        }
        if set_confirm.is_some() && pending_confirm.is_none() {
            *pending_confirm = set_confirm;
        }

        ui.add_space(8.0);
    }

    // ── All Properties ────────────────────────────────────────────────────────
    if !dev.properties.is_empty() {
        section_header(ui, "All sysfs Properties");
        egui::Grid::new("all_props_grid")
            .num_columns(2)
            .spacing([12.0, 2.0])
            .striped(true)
            .show(ui, |ui| {
                let mut props: Vec<_> = dev.properties.iter().collect();
                props.sort_by_key(|(k, _)| k.as_str());
                for (key, val) in props {
                    let editable = dev.editable_properties.contains(key);
                    ui.label(
                        RichText::new(key)
                            .monospace()
                            .size(11.5)
                            .color(if editable {
                                Color32::from_rgb(130, 190, 255)
                            } else {
                                Color32::from_rgb(150, 150, 150)
                            }),
                    );
                    ui.label(RichText::new(val).monospace().size(11.5));
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
    }

    // ── Interfaces ────────────────────────────────────────────────────────────
    if !dev.interfaces.is_empty() {
        section_header(ui, "Interfaces");
        egui::Grid::new("ifaces_grid")
            .num_columns(3)
            .spacing([12.0, 2.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("Class").strong());
                ui.label(RichText::new("Protocol").strong());
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
        ui.add_space(8.0);
    }

    // ── Hierarchy ─────────────────────────────────────────────────────────────
    if dev.parent.is_some() || !dev.children.is_empty() {
        section_header(ui, "Hierarchy");
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
                            ui.label(RichText::new(child).monospace().size(11.5));
                        }
                    });
                    ui.end_row();
                }
            });
    }

    ui.add_space(20.0);
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
            .color(Color32::from_rgb(160, 160, 180)),
    );
    ui.label(RichText::new(value).monospace().size(12.0));
    ui.end_row();
}
