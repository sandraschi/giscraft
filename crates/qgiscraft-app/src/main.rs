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
    features: Vec<store::StoredFeature>,
    selected: Option<String>,
    status: String,
    log: Vec<String>,
    next_n: usize,
    map_zoom: f32,
    center_lng: f64,
    center_lat: f64,
    fit_requested: bool,
}

/// Pixels per degree latitude at zoom 1. Longitude is cos-corrected
/// for the view latitude (Vienna: 1 deg lng ~ 0.67 deg lat in km).
const PX_PER_DEG: f32 = 120.0;

fn project(
    lng: f64,
    lat: f64,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> egui::Pos2 {
    let k = PX_PER_DEG * zoom;
    let cos_lat = center_lat.to_radians().cos() as f32;
    let center = rect.center();
    egui::Pos2::new(
        center.x + ((lng - center_lng) as f32 * cos_lat * k),
        center.y - ((lat - center_lat) as f32 * k),
    )
}

fn unproject(
    pos: egui::Pos2,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> (f64, f64) {
    let k = PX_PER_DEG * zoom;
    let cos_lat = center_lat.to_radians().cos() as f32;
    let center = rect.center();
    let lng = if cos_lat.abs() > f32::EPSILON {
        center_lng + ((pos.x - center.x) / k / cos_lat) as f64
    } else {
        center_lng
    };
    let lat = center_lat + ((center.y - pos.y) / k) as f64;
    (lng, lat)
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
            features: Vec::new(),
            selected: None,
            status: "render on".to_string(),
            log: Vec::new(),
            next_n: 1,
            map_zoom: 1.0,
            center_lng: 16.3738,
            center_lat: 48.2082,
            fit_requested: true,
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
            Ok(conn) => {
                match store::list_layers(&conn) {
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
                }
                match store::list_features(&conn, None) {
                    Ok(feats) => self.features = feats,
                    Err(e) => self.push_log(format!("features failed: {e}")),
                }
            }
            Err(e) => self.push_log(format!("db open failed: {e}")),
        }
    }

    fn is_visible(&self, layer_id: &str) -> bool {
        self.layers
            .iter()
            .find(|l| l.id == layer_id)
            .map(|l| l.visible)
            .unwrap_or(false)
    }

    /// Frame visible features with padding. No-op when nothing is visible.
    fn fit_to_data(&mut self, rect: &egui::Rect) {
        if let Some((clng, clat, zoom)) =
            fit_view(&self.features, &self.layers, rect.width(), rect.height())
        {
            self.center_lng = clng;
            self.center_lat = clat;
            self.map_zoom = zoom;
            self.status = "fit".to_string();
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
        if ui.input(|input| input.key_pressed(egui::Key::F5)) {
            self.refresh();
            self.push_log("refreshed (F5)".to_string());
        }
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
            ui.horizontal(|ui| {
                if ui.button("Fit data").clicked() {
                    self.fit_requested = true;
                }
                if ui.button("Refresh (F5)").clicked() {
                    self.refresh();
                    self.push_log("refreshed".to_string());
                }
            });
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

        // Center: map canvas. Ctrl+wheel zooms to cursor (QGIS muscle
        // memory); drag pans; Fit data frames visible features.
        egui::CentralPanel::default().show(ui, |ui| {
            let shown: usize = self
                .features
                .iter()
                .filter(|f| self.is_visible(f.layer_id.as_str()))
                .count();
            ui.heading(format!("Map canvas ({shown} points, zoom: Ctrl+wheel)"));
            let (resp, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = resp.rect;
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0x05, 0x05, 0x07));
            if self.fit_requested {
                self.fit_requested = false;
                self.fit_to_data(&rect);
            }
            // Crosshair at view center so zoom has a visible anchor.
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
            // Points: cyan, selected layer orange, off-canvas culled.
            for feat in &self.features {
                if !self.is_visible(feat.layer_id.as_str()) {
                    continue;
                }
                let pos = project(
                    feat.lng,
                    feat.lat,
                    self.center_lng,
                    self.center_lat,
                    self.map_zoom,
                    &rect,
                );
                if !rect.contains(pos) {
                    continue;
                }
                let color = if self.selected.as_ref() == Some(&feat.layer_id) {
                    egui::Color32::from_rgb(0xff, 0xb0, 0x26)
                } else {
                    egui::Color32::from_rgb(0x4c, 0xc9, 0xf0)
                };
                painter.circle_filled(pos, 4.0, color);
            }
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(0x60)),
                egui::StrokeKind::Inside,
            );
            // eframe feeds wheel + trackpad into the smooth channel;
            // raw deltas are not exposed on this egui version.
            let (ctrl, scroll, hover) = ui.input(|input| {
                (
                    input.modifiers.ctrl,
                    input.smooth_scroll_delta().y,
                    input.pointer.hover_pos(),
                )
            });
            if resp.hovered() && ctrl && scroll != 0.0 {
                // Zoom to cursor: anchor the geo under the pointer.
                let anchor = hover.unwrap_or(rect.center());
                let (glng, glat) = unproject(
                    anchor,
                    self.center_lng,
                    self.center_lat,
                    self.map_zoom,
                    &rect,
                );
                self.map_zoom = (self.map_zoom * (1.0 + scroll * 0.001)).clamp(0.25, 64.0);
                let k = PX_PER_DEG * self.map_zoom;
                let cos_lat = self.center_lat.to_radians().cos() as f32;
                let c = rect.center();
                if cos_lat.abs() > f32::EPSILON {
                    self.center_lng = glng - ((anchor.x - c.x) / k / cos_lat) as f64;
                }
                self.center_lat = glat + ((anchor.y - c.y) / k) as f64;
                self.status = "zoom".to_string();
            }
            if resp.dragged() {
                let delta = resp.drag_delta();
                let k = PX_PER_DEG * self.map_zoom;
                let cos_lat = self.center_lat.to_radians().cos() as f32;
                if cos_lat.abs() > f32::EPSILON {
                    self.center_lng -= (delta.x / k / cos_lat) as f64;
                }
                self.center_lat += (delta.y / k) as f64;
                self.status = "pan".to_string();
            }
        });
    }
}

/// Fit visible features into a WxH view. Shared by the canvas and the
/// headless BMP preview so pixels and preview agree. None when empty.
fn fit_view(
    features: &[store::StoredFeature],
    layers: &[LayerRow],
    width: f32,
    height: f32,
) -> Option<(f64, f64, f32)> {
    let visible = |id: &str| {
        layers
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.visible)
            .unwrap_or(false)
    };
    let mut min_lng = f64::INFINITY;
    let mut max_lng = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for feat in features {
        if !visible(feat.layer_id.as_str()) {
            continue;
        }
        min_lng = min_lng.min(feat.lng);
        max_lng = max_lng.max(feat.lng);
        min_lat = min_lat.min(feat.lat);
        max_lat = max_lat.max(feat.lat);
    }
    if min_lng.is_infinite() {
        return None;
    }
    let clng = (min_lng + max_lng) / 2.0;
    let clat = (min_lat + max_lat) / 2.0;
    // 20% padding so edge points never sit under the frame.
    let span_lng = ((max_lng - min_lng) * 1.2).max(0.002);
    let span_lat = ((max_lat - min_lat) * 1.2).max(0.002);
    let cos_lat = clat.to_radians().cos() as f32;
    let kx = width / (span_lng as f32 * cos_lat * PX_PER_DEG);
    let ky = height / (span_lat as f32 * PX_PER_DEG);
    Some((clng, clat, kx.min(ky).clamp(0.25, 64.0)))
}

/// Headless BMP preview (24-bit, no deps): same `project` math as the
/// canvas, dark bg, cyan points. Lets a windowless session see the map.
fn render_preview(
    features: &[store::StoredFeature],
    layers: &[LayerRow],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("preview size must be 1..4096".to_string());
    }
    let rect = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::Vec2::new(width as f32, height as f32),
    );
    let (clng, clat, zoom) =
        fit_view(features, layers, width as f32, height as f32).unwrap_or((16.3738, 48.2082, 1.0));
    // BGR pixels, near-black bg.
    let mut px = vec![8_u8; (width as usize) * (height as usize) * 3];
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 3) as usize;
            px[i] = 7;
            px[i + 1] = 5;
            px[i + 2] = 5;
        }
    }
    let plot = |px: &mut [u8], x: i32, y: i32, b: u8, g: u8, r: u8| {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        // BMP rows are bottom-up.
        let i = (((height as i32 - 1 - y) as u32 * width + x as u32) * 3) as usize;
        if i + 2 < px.len() {
            px[i] = b;
            px[i + 1] = g;
            px[i + 2] = r;
        }
    };
    for feat in features {
        let vis = layers
            .iter()
            .find(|l| l.id == feat.layer_id)
            .map(|l| l.visible)
            .unwrap_or(false);
        if !vis {
            continue;
        }
        let pos = project(feat.lng, feat.lat, clng, clat, zoom, &rect);
        let cx = pos.x.round() as i32;
        let cy = pos.y.round() as i32;
        // 4px disk, cyan.
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy <= 6 {
                    plot(&mut px, cx + dx, cy + dy, 0xf0, 0xc9, 0x4c);
                }
            }
        }
    }
    // BMP file: 14-byte header + 40-byte DIB + BGR rows (padded to 4).
    let row_stride = (width * 3).div_ceil(4) * 4;
    let img_size = row_stride * height;
    let file_size = 14 + 40 + img_size;
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 54, 0, 0, 0]);
    out.extend_from_slice(&[40, 0, 0, 0]);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&[1, 0, 24, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&img_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let pad = (row_stride - width * 3) as usize;
    for row in 0..height {
        let start = (row * width * 3) as usize;
        out.extend_from_slice(&px[start..start + (width * 3) as usize]);
        out.extend_from_slice(&vec![0_u8; pad]);
    }
    Ok(out)
}
/// one-line summary, exit 0. Opens no window. Also renders a BMP map
/// preview next to the DB temp dir so windowless runs can see the map.
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
    let mut preview_path = std::env::temp_dir();
    preview_path.push(format!("qgiscraft-preview-{}.bmp", std::process::id()));
    match render_preview(&app.features, &app.layers, 1280, 860) {
        Ok(bytes) => match std::fs::write(&preview_path, &bytes) {
            Ok(()) => println!("preview: {}", preview_path.to_string_lossy()),
            Err(e) => println!("preview write failed: {e}"),
        },
        Err(e) => println!("preview render failed: {e}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0))
    }

    #[test]
    fn view_center_projects_to_rect_center() {
        let rect = test_rect();
        let pos = project(16.3738, 48.2082, 16.3738, 48.2082, 1.0, &rect);
        let center = rect.center();
        assert!((pos.x - center.x).abs() < 0.01);
        assert!((pos.y - center.y).abs() < 0.01);
    }

    #[test]
    fn unproject_inverts_project() {
        let rect = test_rect();
        let (lng, lat) = unproject(egui::Pos2::new(450.0, 280.0), 16.3738, 48.2082, 2.0, &rect);
        let back = project(lng, lat, 16.3738, 48.2082, 2.0, &rect);
        assert!((back.x - 450.0).abs() < 0.01);
        assert!((back.y - 280.0).abs() < 0.01);
    }

    #[test]
    fn zoom_scales_linearly() {
        let rect = test_rect();
        let a = project(16.3838, 48.2082, 16.3738, 48.2082, 1.0, &rect);
        let b = project(16.3838, 48.2082, 16.3738, 48.2082, 2.0, &rect);
        let center = rect.center();
        assert!(((b.x - center.x) / (a.x - center.x) - 2.0).abs() < 0.01);
    }

    fn test_layers() -> Vec<LayerRow> {
        vec![LayerRow {
            id: "rx".to_string(),
            name: "Rx".to_string(),
            visible: true,
            features: 2,
        }]
    }

    fn test_features() -> Vec<store::StoredFeature> {
        vec![
            store::StoredFeature {
                layer_id: "rx".to_string(),
                name: "A".to_string(),
                kind: "node".to_string(),
                lng: 16.30,
                lat: 48.20,
            },
            store::StoredFeature {
                layer_id: "rx".to_string(),
                name: "B".to_string(),
                kind: "node".to_string(),
                lng: 16.40,
                lat: 48.25,
            },
        ]
    }

    #[test]
    fn fit_view_centers_data() {
        let fit = fit_view(&test_features(), &test_layers(), 800.0, 600.0);
        assert!(fit.is_some());
        if let Some((clng, clat, zoom)) = fit {
            assert!((clng - 16.35).abs() < 1e-9);
            assert!((clat - 48.225).abs() < 1e-9);
            assert!((0.25..=64.0).contains(&zoom));
        }
    }

    #[test]
    fn fit_view_empty_is_none() {
        assert!(fit_view(&[], &test_layers(), 800.0, 600.0).is_none());
    }

    #[test]
    fn render_preview_is_bmp_and_rejects_hostile_size() {
        assert!(render_preview(&test_features(), &test_layers(), 0, 600).is_err());
        assert!(render_preview(&test_features(), &test_layers(), 5000, 600).is_err());
        let bmp = render_preview(&test_features(), &test_layers(), 320, 200);
        assert!(bmp.is_ok());
        if let Ok(bytes) = bmp {
            assert!(bytes.len() > 54);
            assert_eq!(&bytes[0..2], b"BM");
        }
    }
}
