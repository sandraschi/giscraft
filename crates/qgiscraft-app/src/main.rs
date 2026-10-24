//! L3 native egui clone shell (QGIS muscle-memory layout, in-process core).
//! Track A: menu + Browser/Layers docks + canvas + status bar.
//! No HTTP, no MCP subprocess: calls `qgiscraft_core` directly.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use eframe::egui;

struct QgisCraftApp {
    status: String,
    layers: Vec<String>,
    log: Vec<String>,
}

impl Default for QgisCraftApp {
    fn default() -> Self {
        Self {
            status: "EPSG:4326 | scale 1:25000 | render on".to_string(),
            layers: vec![
                "osm-basemap".to_string(),
                "wien-pharmacies (demo)".to_string(),
            ],
            log: vec!["qgiscraft ready (Phase 0 shell)".to_string()],
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
                ui.label(self.status.clone());
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
            let mut idx = 0_usize;
            while idx < self.layers.len() {
                let mut on = true;
                if let Some(layer) = self.layers.get(idx) {
                    let label = layer.clone();
                    ui.checkbox(&mut on, label.as_str());
                }
                idx += 1;
            }
            if ui.button("Add demo layer").clicked() {
                let n = self.layers.len() + 1;
                self.layers.push(format!("layer-{n}"));
                self.log
                    .push("layer.add via in-process dispatch (stub)".to_string());
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
                    self.log
                        .push(format!("{tool} -> unsupported in Phase 0 (typed error)"));
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
