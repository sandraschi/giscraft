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
}

impl QgisCraftApp {
    fn open_with_seed() -> Self {
        let db_path = store::default_db_path().to_string_lossy().to_string();
        let mut app = Self {
            db_path,
            layers: Vec::new(),
            selected: None,
            status: "EPSG:3857 | scale 1:25000 | render on".to_string(),
            log: Vec::new(),
            next_n: 1,
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
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} | {} layers | {}",
                    self.db_path,
                    self.layers.len(),
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

        // Center: map canvas (Phase 0 placeholder, render lands Phase 1)
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Map canvas (Phase 0 shell)");
            ui.label("Pan: space/middle-drag | Zoom: wheel | Box zoom: shift-drag (Phase 1)");
            let resp = ui.allocate_response(ui.available_size(), egui::Sense::drag());
            if resp.dragged() {
                self.status = "panning (stub)".to_string();
            }
        });
    }
}

fn main() {
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
        Box::new(|_cc| Ok(Box::new(QgisCraftApp::default()))),
    );
    if let Err(e) = result {
        eprintln!("qgiscraft-app exited: {e}");
    }
}
