//! L3 native egui clone shell (QGIS muscle-memory layout, project DB backed).
//! Track A: menu + Browser/Layers docks + canvas + status bar.
//! No HTTP, no MCP subprocess: owns the SQLite project file directly.
//! The DB file IS the project (QGIS `.qgz` equivalent).
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use eframe::egui;
use qgiscraft_store as store;

struct LayerRow {
    id: String,
    name: String,
    visible: bool,
    features: u64,
}

struct QgisCraftApp {
    db_path: String,
    layers: Vec<LayerRow>,
    selected: Option<String>,
    status: String,
    log: Vec<String>,
    next_n: usize,
    map_zoom: f32,
}

/// High-contrast dark: near-black surfaces, bright text/selection.
/// Default egui dark washes out on bright office light; QGIS dailies
/// stare at this canvas all day, so contrast outranks subtlety.
fn high_contrast_dark() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgb(0x0e, 0x0e, 0x11);
    visuals.panel_fill = egui::Color32::from_rgb(0x16, 0x16, 0x1a);
    visuals.extreme_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x07);
    visuals.code_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x07);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x2b, 0x6c, 0xb0);
    visuals.selection.stroke.color = egui::Color32::WHITE;
    visuals.hyperlink_color = egui::Color32::from_rgb(0x7c, 0xb8, 0xff);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_gray(0xe8);
    visuals
}

impl QgisCraftApp {
    fn open_with_seed() -> Self {
        let db_path = store::default_db_path().to_string_lossy().to_string();
        let mut app = Self {
            db_path,
            layers: Vec::new(),
            selected: None,
            status: "render on".to_string(),
            log: Vec::new(),
            next_n: 1,
            map_zoom: 1.0,
        };
        // Seed an empty project the way QGIS opens with a default basemap.
        match store::open_default() {
            Ok(conn) => {
                let empty = match store::list_layers(&conn) {
                    Ok(layers) => layers.is_empty(),
                    Err(_) => false,
                };
                if empty {
                    let seed = store::StoredLayer::new(
                        "osm-basemap".to_string(),
                        "osm-basemap".to_string(),
                        Some("xyz:osm".to_string()),
                        Some("raster".to_string()),
                        Some("EPSG:3857".to_string()),
                    );
                    match store::add_layer(&conn, &seed) {
                        Ok(()) => app.push_log("seeded osm-basemap".to_string()),
                        Err(e) => app.push_log(format!("seed failed: {e}")),
                    }
                }
            }
            Err(e) => app.push_log(format!("db open failed: {e}")),
        }
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        match store::open_default() {
            Ok(conn) => match store::list_layers(&conn) {
                Ok(layers) => {
                    let counts = store::feature_counts(&conn).unwrap_or_default();
                    self.layers = layers
                        .into_iter()
                        .map(|l| {
                            let n = counts
                                .iter()
                                .find(|(id, _)| *id == l.id)
                                .map(|(_, n)| *n)
                                .unwrap_or(0);
                            LayerRow {
                                id: l.id,
                                name: l.name,
                                visible: l.visible,
                                features: n,
                            }
                        })
                        .collect();
                }
                Err(e) => self.push_log(format!("list failed: {e}")),
            },
            Err(e) => self.push_log(format!("db open failed: {e}")),
        }
    }

    fn push_log(&mut self, line: String) {
        self.log.push(line);
        while self.log.len() > 50 {
            self.log.remove(0);
        }
    }
}

impl Default for QgisCraftApp {
    fn default() -> Self {
        Self::open_with_seed()
    }
}

impl eframe::App for QgisCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Menu bar parity: Project/Edit/View/Layer/Settings/Plugins/Vector/Raster/Database/Web/Processing/Help
        egui::Panel::top("menubar").show(ui, |ui| {
            ui.horizontal(|ui| {
                for menu in [
                    "Project",
                    "Edit",
                    "View",
                    "Layer",
                    "Settings",
                    "Plugins",
                    "Vector",
                    "Raster",
                    "Database",
                    "Web",
                    "Processing",
                    "Help",
                ] {
                    ui.menu_button(menu, |ui| {
                        ui.label(format!(
                            "{menu} commands land with Phase 0.1 dispatch wiring"
                        ));
                    });
                }
            });
        });

        // Bottom status + log (QGIS status bar + Log Messages parity)
        egui::Panel::bottom("statusbar").show(ui, |ui| {
            let db_name = std::path::Path::new(&self.db_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.db_path.clone());
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} | EPSG:3857 | 1:{} ({}%) | {}",
                    db_name,
                    (25_000.0 / self.map_zoom).round() as u32,
                    (self.map_zoom * 100.0).round() as u32,
                    self.status.clone()
                ));
                if let Some(last) = self.log.last() {
                    ui.separator();
                    ui.monospace(last.clone());
                }
            });
        });

        // Left: Browser + Layers docks (QGIS default left stack)
        egui::Panel::left("browser_layers").show(ui, |ui| {
            ui.heading("Browser");
            ui.label("XYZ Tiles / GeoPackage / PostGIS / WMS (Phase 0.1)");
            ui.separator();
            ui.heading("Layers");
            let mut toggle: Option<(String, bool)> = None;
            let mut remove_id: Option<String> = None;
            let mut idx = 0_usize;
            while idx < self.layers.len() {
                let (id, name, mut visible, features) = if let Some(row) = self.layers.get(idx) {
                    (row.id.clone(), row.name.clone(), row.visible, row.features)
                } else {
                    idx += 1;
                    continue;
                };
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut visible, name.as_str()).changed() {
                        toggle = Some((id.clone(), visible));
                    }
                    ui.label(format!("({features})"));
                    if ui
                        .selectable_label(self.selected.as_ref() == Some(&id), "sel")
                        .clicked()
                    {
                        self.selected = Some(id.clone());
                    }
                });
                if self.selected.as_ref() == Some(&id) && ui.button("Remove selected").clicked() {
                    remove_id = Some(id.clone());
                }
                idx += 1;
            }
            if let Some((id, visible)) = toggle {
                match store::open_default()
                    .and_then(|c| store::set_visible(&c, id.as_str(), visible))
                {
                    Ok(true) => self.refresh(),
                    Ok(false) => self.push_log(format!("not found: {id}")),
                    Err(e) => self.push_log(format!("visible failed: {e}")),
                }
            }
            if let Some(id) = remove_id {
                match store::open_default().and_then(|c| store::remove_layer(&c, id.as_str())) {
                    Ok(true) => {
                        self.push_log(format!("removed {id}"));
                        self.selected = None;
                        self.refresh();
                    }
                    Ok(false) => self.push_log(format!("not found: {id}")),
                    Err(e) => self.push_log(format!("remove failed: {e}")),
                }
            }
            if ui.button("Add demo layer").clicked() {
                let n = self.next_n;
                self.next_n += 1;
                let layer = store::StoredLayer::new(
                    format!("demo-{n}"),
                    format!("demo-{n}"),
                    Some("osm".to_string()),
                    Some("point".to_string()),
                    None,
                );
                match store::open_default().and_then(|c| store::add_layer(&c, &layer)) {
                    Ok(()) => {
                        self.push_log(format!("layer.add demo-{n}"));
                        self.refresh();
                    }
                    Err(e) => self.push_log(format!("layer.add failed: {e}")),
                }
            }
        });

        // Right: Layer Styling + Processing Toolbox (QGIS default right stack)
        egui::Panel::right("styling_processing").show(ui, |ui| {
            ui.heading("Layer Styling");
            ui.label("Single / Categorized / Graduated (Phase 1)");
            ui.separator();
            ui.heading("Processing Toolbox");
            for tool in ["Buffer", "Clip", "Dissolve", "Reproject", "Hillshade"] {
                if ui.button(tool).clicked() {
                    self.push_log(format!("{tool} -> unsupported in Phase 0 (typed error)"));
                }
            }
        });

        // Center: map canvas. Ctrl+wheel zooms (QGIS muscle memory);
        // plain drag pans the stub view. Render lands Phase 1.
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Map canvas (zoom: Ctrl+wheel)");
            let (resp, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = resp.rect;
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0x05, 0x05, 0x07));
            // Crosshair at zoom-scaled center so zoom has a visible anchor.
            let center = rect.center();
            let arm = 24.0 * self.map_zoom;
            painter.line_segment(
                [
                    center + egui::Vec2::new(-arm, 0.0),
                    center + egui::Vec2::new(arm, 0.0),
                ],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(0x7c, 0xb8, 0xff)),
            );
            painter.line_segment(
                [
                    center + egui::Vec2::new(0.0, -arm),
                    center + egui::Vec2::new(0.0, arm),
                ],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(0x7c, 0xb8, 0xff)),
            );
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(0x60)),
                egui::StrokeKind::Inside,
            );
            // eframe feeds wheel + trackpad into the smooth channel;
            // raw deltas are not exposed on this egui version.
            let (ctrl, scroll) =
                ui.input(|input| (input.modifiers.ctrl, input.smooth_scroll_delta().y));
            if resp.hovered() && ctrl && scroll != 0.0 {
                self.map_zoom = (self.map_zoom * (1.0 + scroll * 0.001)).clamp(0.25, 8.0);
                self.status = "zoom".to_string();
            }
            if resp.dragged() {
                self.status = "panning (stub)".to_string();
            }
        });
    }
}

/// Headless parity path for CI: open/seed/list the project DB, print a
/// one-line summary, exit 0. Opens no window. Mirrors the devices-app pattern.
fn smoke() {
    let app = QgisCraftApp::open_with_seed();
    let total_features: u64 = app.layers.iter().map(|l| l.features).sum();
    println!(
        "qgiscraft smoke: {} layers, {} features, db={}",
        app.layers.len(),
        total_features,
        app.db_path
    );
    for layer in &app.layers {
        println!(
            "  layer {} visible={} features={}",
            layer.id, layer.visible, layer.features
        );
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--smoke") {
        return smoke();
    }
    let viewport = egui::ViewportBuilder::default()
        .with_title("qgiscraft")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([900.0, 600.0]);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "qgiscraft",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(high_contrast_dark());
            Ok(Box::new(QgisCraftApp::open_with_seed()))
        }),
    );
    if let Err(e) = result {
        eprintln!("qgiscraft-app exited: {e}");
    }
}
