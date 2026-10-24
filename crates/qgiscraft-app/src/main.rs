//! L3 native egui clone shell (QGIS muscle-memory layout, in-process core).
//! Track A: menu + Browser/Layers docks + canvas + status bar.
//! No HTTP, no MCP subprocess: owns a real `Project` in-process.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use eframe::egui;
use qgiscraft_core::project::{Layer, Project};

struct QgisCraftApp {
    project: Project,
    selected: Option<String>,
    status: String,
    log: Vec<String>,
    next_n: usize,
}

impl Default for QgisCraftApp {
    fn default() -> Self {
        let mut project = Project::new("wien-demo".to_string());
        let seed = Layer {
            id: "osm-basemap".to_string(),
            name: "osm-basemap".to_string(),
            source: "xyz:osm".to_string(),
            geom_type: "raster".to_string(),
            crs: "EPSG:3857".to_string(),
            visible: true,
        };
        let mut log = Vec::new();
        match project.add_layer(seed) {
            Ok(()) => log.push("qgiscraft ready (seeded osm-basemap)".to_string()),
            Err(e) => log.push(format!("seed failed: {e}")),
        }
        Self {
            project,
            selected: None,
            status: "EPSG:3857 | scale 1:25000 | render on".to_string(),
            log,
            next_n: 1,
        }
    }
}

impl QgisCraftApp {
    fn push_log(&mut self, line: String) {
        self.log.push(line);
        while self.log.len() > 50 {
            self.log.remove(0);
        }
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
                    self.project.crs,
                    self.project.layers.len(),
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
            let mut remove_id: Option<String> = None;
            let mut idx = 0_usize;
            while idx < self.project.layers.len() {
                let (id, name, mut visible) = if let Some(layer) = self.project.layers.get(idx) {
                    (layer.id.clone(), layer.name.clone(), layer.visible)
                } else {
                    idx += 1;
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.checkbox(&mut visible, name.as_str());
                    if ui
                        .selectable_label(self.selected.as_ref() == Some(&id), "sel")
                        .clicked()
                    {
                        self.selected = Some(id.clone());
                    }
                });
                if let Some(layer) = self.project.layers.get_mut(idx) {
                    layer.visible = visible;
                }
                if self.selected.as_ref() == Some(&id) && ui.button("Remove selected").clicked() {
                    remove_id = Some(id.clone());
                }
                idx += 1;
            }
            if let Some(id) = remove_id {
                match self.project.remove_layer(id.as_str()) {
                    Ok(true) => {
                        self.push_log(format!("removed {id}"));
                        self.selected = None;
                    }
                    Ok(false) => self.push_log(format!("not found: {id}")),
                    Err(e) => self.push_log(format!("remove failed: {e}")),
                }
            }
            if ui.button("Add demo layer").clicked() {
                let n = self.next_n;
                self.next_n += 1;
                let layer = Layer {
                    id: format!("demo-{n}"),
                    name: format!("demo-{n}"),
                    source: "osm".to_string(),
                    geom_type: "point".to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                };
                match self.project.add_layer(layer) {
                    Ok(()) => self.push_log(format!("layer.add demo-{n}")),
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
