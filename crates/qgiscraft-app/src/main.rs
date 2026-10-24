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
use qgiscraft_core::control;
use qgiscraft_core::tiles;
use qgiscraft_core::triangulate;
use qgiscraft_store as store;
use std::collections::HashMap;

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
    zoom: f32,
    center_lng: f64,
    center_lat: f64,
    fit_requested: bool,
    fit_only: Option<String>,
    confirm_new: bool,
    show_about: bool,
    show_settings: bool,
    textures: HashMap<(u8, i64, i64), egui::TextureHandle>,
    control_rx: Option<std::sync::mpsc::Receiver<control::ControlRequest>>,
    last_rect: Option<egui::Rect>,
}

/// Pixels per world at zoom 0. Float zoom behaves like a slippy map:
/// zoom+1 doubles the scale, matching XYZ tile zooms.
fn world_px(zoom: f32) -> f64 {
    256.0 * 2_f64.powf(zoom as f64)
}

fn merc_x(lng: f64, world: f64) -> f64 {
    world * (lng + 180.0) / 360.0
}

fn merc_y(lat: f64, world: f64) -> f64 {
    let r = lat.clamp(-85.05112878, 85.05112878).to_radians();
    world * (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0
}

fn merc_lng(x: f64, world: f64) -> f64 {
    x / world * 360.0 - 180.0
}

fn merc_lat(y: f64, world: f64) -> f64 {
    let t = std::f64::consts::PI * (1.0 - 2.0 * y / world);
    t.sinh().atan().to_degrees()
}

fn project(
    lng: f64,
    lat: f64,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> egui::Pos2 {
    let world = world_px(zoom);
    let center = rect.center();
    egui::Pos2::new(
        center.x + (merc_x(lng, world) - merc_x(center_lng, world)) as f32,
        center.y + (merc_y(lat, world) - merc_y(center_lat, world)) as f32,
    )
}

fn unproject(
    pos: egui::Pos2,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> (f64, f64) {
    let world = world_px(zoom);
    let center = rect.center();
    let x = merc_x(center_lng, world) + (pos.x - center.x) as f64;
    let y = merc_y(center_lat, world) + (pos.y - center.y) as f64;
    (merc_lng(x, world), merc_lat(y, world))
}

/// High-contrast dark: near-black surfaces, white text, bright accents.
/// No grey-on-black anywhere: body text, strokes and badges are full white
/// or saturated color. QGIS dailies stare at this canvas all day, so
/// contrast outranks subtlety.
fn high_contrast_dark() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::WHITE);
    visuals.window_fill = egui::Color32::from_rgb(0x0e, 0x0e, 0x11);
    visuals.panel_fill = egui::Color32::from_rgb(0x16, 0x16, 0x1a);
    visuals.extreme_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x07);
    visuals.code_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x07);
    visuals.faint_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x07);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x2b, 0x6c, 0xb0);
    visuals.selection.stroke.color = egui::Color32::WHITE;
    visuals.hyperlink_color = egui::Color32::from_rgb(0x7c, 0xb8, 0xff);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    visuals
}

impl QgisCraftApp {
    fn open_with_seed(
        control_rx: Option<std::sync::mpsc::Receiver<control::ControlRequest>>,
    ) -> Self {
        let db_path = store::default_db_path().to_string_lossy().to_string();
        let mut app = Self {
            db_path,
            layers: Vec::new(),
            features: Vec::new(),
            selected: None,
            status: "render on".to_string(),
            log: Vec::new(),
            next_n: 1,
            zoom: 15.0,
            center_lng: 16.3738,
            center_lat: 48.2082,
            fit_requested: true,
            fit_only: None,
            confirm_new: false,
            show_about: false,
            show_settings: false,
            textures: HashMap::new(),
            control_rx,
            last_rect: None,
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
}

/// Deterministic per-layer color from a fixed high-contrast palette.
/// Same layer id always maps to the same color (canvas + preview agree).
fn layer_color(id: &str) -> egui::Color32 {
    const PALETTE: [[u8; 3]; 8] = [
        [0x4c, 0xc9, 0xf0],
        [0xff, 0xb0, 0x26],
        [0x80, 0xed, 0x99],
        [0xff, 0x70, 0xa6],
        [0xff, 0xe0, 0x66],
        [0xc7, 0x7d, 0xff],
        [0xff, 0x59, 0x64],
        [0x64, 0xdf, 0xdf],
    ];
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let c = PALETTE[(h as usize) % PALETTE.len()];
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

/// Drawable geometry parsed from a feature's `geom_json`.
/// Points (None) fall back to the centroid dot.
enum Geom {
    Line(Vec<(f64, f64)>),
    Poly(Vec<(f64, f64)>),
}

fn parse_geom(feat: &store::StoredFeature) -> Option<Geom> {
    let raw = feat.geom_json.as_ref()?;
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let ty = v.get("type").and_then(|t| t.as_str())?;
    let rings = v.get("coordinates")?;
    let first_ring = if ty == "Polygon" {
        rings.get(0)?
    } else if ty == "LineString" {
        rings
    } else {
        return None;
    };
    let arr = first_ring.as_array()?;
    let mut pts = Vec::new();
    for pt in arr {
        let lng = pt.get(0).and_then(|n| n.as_f64());
        let lat = pt.get(1).and_then(|n| n.as_f64());
        match (lng, lat) {
            (Some(lng), Some(lat))
                if (-180.0..=180.0).contains(&lng) && (-90.0..=90.0).contains(&lat) =>
            {
                pts.push((lng, lat))
            }
            _ => continue,
        }
    }
    if ty == "Polygon" {
        if pts.len() < 3 {
            return None;
        }
        // Auto-close unclosed rings (renderers, including QGIS, do this).
        if pts.first() != pts.last() {
            if let Some(first) = pts.first().copied() {
                pts.push(first);
            }
        }
        Some(Geom::Poly(pts))
    } else {
        if pts.len() < 2 {
            return None;
        }
        Some(Geom::Line(pts))
    }
}

/// Frame visible features with padding. No-op when nothing is visible.
impl QgisCraftApp {
    /// Zoom to cursor: anchor the geo under the pointer, then move.
    fn zoom_to(&mut self, anchor: egui::Pos2, target: f32, rect: &egui::Rect) {
        let (glng, glat) = unproject(anchor, self.center_lng, self.center_lat, self.zoom, rect);
        self.zoom = target.clamp(2.0, 19.0);
        let world = world_px(self.zoom);
        let c = rect.center();
        self.center_lng = merc_lng(merc_x(glng, world) - (anchor.x - c.x) as f64, world);
        self.center_lat = merc_lat(merc_y(glat, world) - (anchor.y - c.y) as f64, world);
        self.status = "zoom".to_string();
    }

    /// Frame visible features (or just `fit_only` for Zoom to layer).
    fn fit_to_data(&mut self, rect: &egui::Rect) {
        let only = self.fit_only.clone();
        if let Some((clng, clat, zoom)) = fit_view(
            &self.features,
            &self.layers,
            rect.width(),
            rect.height(),
            only.as_deref(),
        ) {
            self.center_lng = clng;
            self.center_lat = clat;
            self.zoom = zoom;
            self.status = "fit".to_string();
        }
        self.fit_only = None;
    }

    /// Project > New: wipe all layers/features, reseed the basemap.
    /// The menu arms this (double-click); the function itself just runs.
    fn new_project(&mut self) {
        let outcome = match store::open_default() {
            Ok(conn) => match store::clear_all(&conn) {
                Ok(n) => {
                    let seed = store::StoredLayer::new(
                        "osm-basemap".to_string(),
                        "osm-basemap".to_string(),
                        Some("xyz:osm".to_string()),
                        Some("raster".to_string()),
                        Some("EPSG:3857".to_string()),
                    );
                    match store::add_layer(&conn, &seed) {
                        Ok(()) => format!("new project ({n} layers cleared)"),
                        Err(e) => format!("seed failed: {e}"),
                    }
                }
                Err(e) => format!("new failed: {e}"),
            },
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = None;
        self.confirm_new = false;
        self.refresh();
        self.push_log(outcome);
    }

    /// Layer > Zoom to layer: fit the selected layer's features.
    fn zoom_to_selected(&mut self) {
        if let Some(id) = self.selected.clone() {
            self.fit_only = Some(id);
            self.fit_requested = true;
            self.push_log("zoom to layer".to_string());
        } else {
            self.push_log("nothing selected".to_string());
        }
    }

    /// Show/hide every layer at once.
    fn set_all_visible(&mut self, visible: bool) {
        let ids: Vec<String> = self.layers.iter().map(|l| l.id.clone()).collect();
        let mut done = 0_usize;
        for id in ids {
            let ok = match store::open_default() {
                Ok(conn) => store::set_visible(&conn, id.as_str(), visible).unwrap_or(false),
                Err(_) => false,
            };
            if ok {
                done += 1;
            }
        }
        self.refresh();
        self.push_log(format!(
            "{} {done} layers",
            if visible { "showed" } else { "hid" }
        ));
    }

    /// Remove the selected layer (dock button, Layer menu, Delete key).
    fn remove_selected(&mut self) {
        if let Some(id) = self.selected.clone() {
            match store::open_default().and_then(|c| store::remove_layer(&c, id.as_str())) {
                Ok(true) => {
                    self.push_log(format!("removed {id}"));
                    self.selected = None;
                    self.refresh();
                }
                Ok(false) => self.push_log(format!("not found: {id}")),
                Err(e) => self.push_log(format!("remove failed: {e}")),
            }
        } else {
            self.push_log("nothing selected".to_string());
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
        Self::open_with_seed(None)
    }
}

impl QgisCraftApp {
    /// Poll the control channel (called every frame when `--control` runs).
    /// Drains first, executes after, so no borrow of self overlaps the queue.
    fn poll_control(&mut self) {
        let mut pending = Vec::new();
        if let Some(rx) = self.control_rx.as_ref() {
            while let Ok(req) = rx.try_recv() {
                let control::ControlRequest {
                    method,
                    params,
                    respond,
                    ..
                } = req;
                pending.push((method, params, respond));
            }
        }
        for (method, params, respond) in pending {
            let reply = self.execute_control(method.as_str(), &params);
            let _ = respond.send(reply);
        }
    }

    /// Execute one validated control request against live app state.
    /// Pure dispatch: every arm returns data, never panics.
    fn execute_control(&mut self, method: &str, params: &serde_json::Value) -> serde_json::Value {
        let str_param = |key: &str| {
            params
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        let opt_str = |key: &str| {
            params
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };
        match method {
            "project.status" => serde_json::json!({
                "db": self.db_path,
                "crs": "EPSG:3857",
                "layers": self.layers.len(),
                "center": [self.center_lng, self.center_lat],
                "zoom": self.zoom,
                "selected": self.selected,
            }),
            "layer.add" => {
                let id = str_param("id");
                let name = str_param("name");
                if id.trim().is_empty() || name.trim().is_empty() {
                    return serde_json::json!({"error": "invalid params: id and name required"});
                }
                let layer = store::StoredLayer::new(
                    id,
                    name,
                    opt_str("source"),
                    opt_str("geom_type"),
                    opt_str("crs"),
                );
                match store::open_default().and_then(|c| store::add_layer(&c, &layer)) {
                    Ok(()) => {
                        self.refresh();
                        serde_json::json!({"added": layer.id})
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            "layer.list" => match store::open_default() {
                Ok(conn) => match store::list_layers(&conn) {
                    Ok(layers) => {
                        let counts = store::feature_counts(&conn).unwrap_or_default();
                        let rows: Vec<serde_json::Value> = layers
                            .into_iter()
                            .map(|l| {
                                let n = counts
                                    .iter()
                                    .find(|(id, _)| *id == l.id)
                                    .map(|(_, n)| *n)
                                    .unwrap_or(0);
                                serde_json::json!({"id": l.id, "name": l.name, "visible": l.visible, "features": n})
                            })
                            .collect();
                        serde_json::json!({"count": rows.len(), "layers": rows})
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                },
                Err(e) => serde_json::json!({"error": e.to_string()}),
            },
            "layer.remove" => {
                let id = str_param("id");
                if id.trim().is_empty() {
                    return serde_json::json!({"error": "invalid params: id required"});
                }
                match store::open_default().and_then(|c| store::remove_layer(&c, id.as_str())) {
                    Ok(removed) => {
                        if removed {
                            self.selected = None;
                            self.refresh();
                        }
                        serde_json::json!({"id": id, "removed": removed})
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            "ui.inspect" => {
                let canvas = self
                    .last_rect
                    .map(|r| serde_json::json!({"w": r.width(), "h": r.height()}));
                serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "center": [self.center_lng, self.center_lat],
                    "zoom": self.zoom,
                    "selected": self.selected,
                    "layers": self.layers.len(),
                    "db": self.db_path,
                    "canvas": canvas,
                })
            }
            "map.fit" => {
                let only = opt_str("layer_id");
                self.fit_only = only.filter(|s| !s.trim().is_empty());
                self.fit_requested = true;
                serde_json::json!({"queued": true})
            }
            "map.render" => {
                let w = params
                    .get("width")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(1280)
                    .clamp(64, 2048);
                let h = params
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u32::try_from(n).ok())
                    .unwrap_or(860)
                    .clamp(64, 2048);
                let with_tiles = params
                    .get("tiles")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                match store::open_default() {
                    Ok(conn) => {
                        let layers = store::list_layers(&conn).unwrap_or_default();
                        let feats = store::list_features(&conn, None).unwrap_or_default();
                        let rows: Vec<LayerRow> = layers
                            .into_iter()
                            .map(|l| {
                                let n = store::feature_counts(&conn)
                                    .unwrap_or_default()
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
                        match render_preview(&feats, &rows, w, h, with_tiles) {
                            Ok(bytes) => {
                                let mut path = std::env::temp_dir();
                                path.push(format!("qgiscraft-control-{}.bmp", std::process::id()));
                                match std::fs::write(&path, &bytes) {
                                    Ok(()) => serde_json::json!({"path": path.to_string_lossy()}),
                                    Err(e) => serde_json::json!({"error": format!("write: {e}")}),
                                }
                            }
                            Err(e) => serde_json::json!({"error": e}),
                        }
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            _ => serde_json::json!({"error": format!("unknown method: {method}")}),
        }
    }
}

impl eframe::App for QgisCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_control();
        // Shortcuts: F5 refresh, Delete removes selection, Ctrl+Plus/Minus/0 zoom.
        if ui.input(|input| input.key_pressed(egui::Key::F5)) {
            self.refresh();
            self.push_log("refreshed (F5)".to_string());
        }
        if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            self.remove_selected();
        }
        let (ctrl, plus, minus, zero) = ui.input(|input| {
            (
                input.modifiers.ctrl,
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
                input.key_pressed(egui::Key::Minus),
                input.key_pressed(egui::Key::Num0),
            )
        });
        if ctrl && plus {
            self.zoom = (self.zoom + 1.0).clamp(2.0, 19.0);
            self.status = "zoom".to_string();
        }
        if ctrl && minus {
            self.zoom = (self.zoom - 1.0).clamp(2.0, 19.0);
            self.status = "zoom".to_string();
        }
        if ctrl && zero {
            self.fit_only = None;
            self.fit_requested = true;
        }
        // Menu bar parity: Project/Edit/View/Layer/Settings/Plugins/Vector/Raster/Database/Web/Processing/Help
        egui::Panel::top("menubar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Project", |ui| {
                    let label = if self.confirm_new {
                        "New (click again to confirm wipe)"
                    } else {
                        "New (clears project)"
                    };
                    if ui.button(label).clicked() {
                        if self.confirm_new {
                            self.new_project();
                        } else {
                            self.confirm_new = true;
                            self.push_log("New armed: click again to wipe".to_string());
                        }
                    }
                    if ui.button("Refresh (F5)").clicked() {
                        self.confirm_new = false;
                        self.refresh();
                        self.push_log("refreshed".to_string());
                    }
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Layer", |ui| {
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
                    if ui.button("Remove selected (Del)").clicked() {
                        self.remove_selected();
                    }
                    if ui.button("Zoom to selected").clicked() {
                        self.zoom_to_selected();
                    }
                    if ui.button("Show all").clicked() {
                        self.set_all_visible(true);
                    }
                    if ui.button("Hide all").clicked() {
                        self.set_all_visible(false);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Zoom in (Ctrl++)").clicked() {
                        self.zoom = (self.zoom + 1.0).clamp(2.0, 19.0);
                        self.status = "zoom".to_string();
                    }
                    if ui.button("Zoom out (Ctrl+-)").clicked() {
                        self.zoom = (self.zoom - 1.0).clamp(2.0, 19.0);
                        self.status = "zoom".to_string();
                    }
                    if ui.button("Fit data (Ctrl+0)").clicked() {
                        self.fit_only = None;
                        self.fit_requested = true;
                    }
                    if ui.button("Reset view (Wien)").clicked() {
                        self.center_lng = 16.3738;
                        self.center_lat = 48.2082;
                        self.zoom = 15.0;
                        self.status = "reset".to_string();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Configuration...").clicked() {
                        self.show_settings = true;
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About qgiscraft").clicked() {
                        self.show_about = true;
                    }
                });
                for menu in [
                    "Edit",
                    "Plugins",
                    "Vector",
                    "Raster",
                    "Database",
                    "Web",
                    "Processing",
                ] {
                    ui.menu_button(menu, |ui| {
                        ui.label(format!(
                            "{menu} commands land with later phases (typed, not stubbed)"
                        ));
                    });
                }
            });
        });
        if self.show_about {
            egui::Window::new("About qgiscraft").show(ui.ctx(), |ui| {
                ui.label(format!("qgiscraft-mcp {}", env!("CARGO_PKG_VERSION")));
                ui.label("QGIS-class GIS: 1:1 clone shell, modern Rust core.");
                ui.label("CLI + MCP (stdio) + native egui, one command bus.");
                ui.label("Tiles (c) OpenStreetMap contributors (ODbL).");
                ui.label("Workflow: overpass.query --live --ingest, then Fit data.");
                if ui.button("Close").clicked() {
                    self.show_about = false;
                }
            });
        }
        if self.show_settings {
            egui::Window::new("Configuration").show(ui.ctx(), |ui| {
                ui.label("Effective endpoints (read-only this session):");
                ui.monospace(format!("project db : {}", self.db_path));
                ui.monospace(format!(
                    "tiles      : {}",
                    qgiscraft_core::tiles::tile_url_template()
                ));
                ui.monospace(format!(
                    "photon     : {}",
                    qgiscraft_core::geocode::photon_base()
                ));
                ui.monospace(format!(
                    "overpass   : {}",
                    qgiscraft_core::overpass::overpass_url()
                ));
                ui.label("Override via environment before launch:");
                ui.monospace("QGISCRAFT_DB / QGISCRAFT_TILES_URL / QGISCRAFT_PHOTON_URL / QGISCRAFT_OVERPASS_URL");
                ui.label("Restart the app to apply changes.");
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
        }

        // Bottom status + log (QGIS status bar + Log Messages parity)
        egui::Panel::bottom("statusbar").show(ui, |ui| {
            let db_name = std::path::Path::new(&self.db_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.db_path.clone());
            ui.horizontal(|ui| {
                // Web Mercator scale at 96 dpi: meters/px * px/m.
                let mpp = 156543.03392 * self.center_lat.to_radians().cos()
                    / 2_f64.powf(self.zoom as f64);
                ui.label(format!(
                    "{} | EPSG:3857 | z{:.1} 1:{} | {}",
                    db_name,
                    self.zoom,
                    (mpp * 3779.527).round() as u64,
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

        // Center: map canvas. XYZ tiles under vector points; Ctrl+wheel
        // zooms to cursor (slippy zoom, QGIS muscle memory); drag pans.
        egui::CentralPanel::default().show(ui, |ui| {
            let shown: usize = self
                .features
                .iter()
                .filter(|f| self.is_visible(f.layer_id.as_str()))
                .count();
            ui.heading(format!(
                "Map canvas ({shown} points, z{:.1}, zoom: wheel)",
                self.zoom
            ));
            let (resp, painter) =
                ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let rect = resp.rect;
            self.last_rect = Some(rect);
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(0x05, 0x05, 0x07));
            if self.fit_requested {
                self.fit_requested = false;
                self.fit_to_data(&rect);
            }
            // XYZ underlay: integer tile zoom nearest the float view zoom.
            // Cache-first sync fetch, max 2 network misses per frame so the
            // UI paints in progressively instead of freezing (async worker
            // is the follow-up; see gaps).
            let tile_z = (self.zoom.floor() as i64).clamp(0, 19) as u8;
            let (wlng, nlat) =
                unproject(rect.min, self.center_lng, self.center_lat, self.zoom, &rect);
            let (elng, slat) =
                unproject(rect.max, self.center_lng, self.center_lat, self.zoom, &rect);
            match tiles::bbox_to_tiles(wlng, slat, elng, nlat, tile_z) {
                Ok(list) => {
                    let mut fetched = 0_usize;
                    for t in list {
                        let key = (t.z, t.x, t.y);
                        if !self.textures.contains_key(&key) && fetched >= 2 {
                            continue;
                        }
                        let loaded = match self.textures.entry(key) {
                            std::collections::hash_map::Entry::Occupied(_) => true,
                            std::collections::hash_map::Entry::Vacant(slot) => {
                                fetched += 1;
                                match tiles::fetch_tile_png(t)
                                    .and_then(|png| tiles::decode_png_rgba(&png))
                                {
                                    Ok((w, h, rgba)) => {
                                        let img = egui::ColorImage::from_rgba_unmultiplied(
                                            [w as usize, h as usize],
                                            &rgba,
                                        );
                                        let handle = ui.ctx().load_texture(
                                            format!("tile_{}_{}_{}", t.z, t.x, t.y),
                                            img,
                                            egui::TextureOptions::LINEAR,
                                        );
                                        slot.insert(handle);
                                        true
                                    }
                                    Err(fetch_err) => {
                                        self.status = format!("tile {fetch_err}");
                                        false
                                    }
                                }
                            }
                        };
                        if !loaded {
                            continue;
                        }
                        if let Some(handle) = self.textures.get(&key) {
                            let (x0, y0, x1, y1) = tiles::tile_bounds(t);
                            let a =
                                project(x0, y1, self.center_lng, self.center_lat, self.zoom, &rect);
                            let b =
                                project(x1, y0, self.center_lng, self.center_lat, self.zoom, &rect);
                            painter.image(
                                handle.id(),
                                egui::Rect::from_min_max(a, b),
                                egui::Rect::from_min_max(
                                    egui::Pos2::ZERO,
                                    egui::Pos2::new(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
                Err(e) => {
                    self.status = format!("tiles: {e}");
                }
            }
            // Geometry: polygons filled, lines stroked, points dotted.
            // Per-layer palette; selected layer also gets name labels.
            for feat in &self.features {
                if !self.is_visible(feat.layer_id.as_str()) {
                    continue;
                }
                let color = layer_color(feat.layer_id.as_str());
                let selected = self.selected.as_ref() == Some(&feat.layer_id);
                match parse_geom(feat) {
                    Some(Geom::Poly(ring)) => {
                        // Ear-clipped fill (concave-safe) + outline.
                        let tris = triangulate::earcut_tris(&ring);
                        let mut mesh = egui::Mesh::default();
                        let fill = egui::Color32::from_rgba_unmultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            70,
                        );
                        for t in tris.as_chunks::<3>().0 {
                            let [ia, ib, ic] = *t;
                            for idx in [ia, ib, ic] {
                                if let Some(&(lng, lat)) = ring.get(idx) {
                                    mesh.colored_vertex(
                                        project(
                                            lng,
                                            lat,
                                            self.center_lng,
                                            self.center_lat,
                                            self.zoom,
                                            &rect,
                                        ),
                                        fill,
                                    );
                                }
                            }
                        }
                        painter.add(mesh);
                        let pts: Vec<egui::Pos2> = ring
                            .iter()
                            .map(|(lng, lat)| {
                                project(
                                    *lng,
                                    *lat,
                                    self.center_lng,
                                    self.center_lat,
                                    self.zoom,
                                    &rect,
                                )
                            })
                            .collect();
                        if pts.iter().any(|p| rect.contains(*p)) {
                            painter
                                .add(egui::Shape::closed_line(pts, egui::Stroke::new(2.0, color)));
                        }
                    }
                    Some(Geom::Line(line)) => {
                        let pts: Vec<egui::Pos2> = line
                            .iter()
                            .map(|(lng, lat)| {
                                project(
                                    *lng,
                                    *lat,
                                    self.center_lng,
                                    self.center_lat,
                                    self.zoom,
                                    &rect,
                                )
                            })
                            .collect();
                        if pts.iter().any(|p| rect.contains(*p)) {
                            painter.add(egui::Shape::line(pts, egui::Stroke::new(2.0, color)));
                        }
                    }
                    None => {
                        let pos = project(
                            feat.lng,
                            feat.lat,
                            self.center_lng,
                            self.center_lat,
                            self.zoom,
                            &rect,
                        );
                        if !rect.contains(pos) {
                            continue;
                        }
                        painter.circle_filled(pos, 4.0, color);
                    }
                }
                if selected {
                    let pos = project(
                        feat.lng,
                        feat.lat,
                        self.center_lng,
                        self.center_lat,
                        self.zoom,
                        &rect,
                    );
                    if rect.contains(pos) {
                        painter.text(
                            pos + egui::Vec2::new(6.0, -6.0),
                            egui::Align2::LEFT_BOTTOM,
                            feat.name.clone(),
                            egui::FontId::proportional(11.0),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );
            painter.text(
                rect.right_bottom() + egui::Vec2::new(-8.0, -20.0),
                egui::Align2::RIGHT_BOTTOM,
                "(c) OpenStreetMap contributors",
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
            // Wheel zoom (QGIS default: plain wheel zooms; Ctrl+wheel and
            // pinch arrive as a multiplicative factor - egui routes
            // Ctrl+scroll into zoom_delta and leaves the scroll channel
            // empty by design, so both paths must be read).
            let (zoom_factor, scroll, hover) = ui.input(|input| {
                (
                    input.zoom_delta(),
                    input.smooth_scroll_delta().y,
                    input.pointer.hover_pos(),
                )
            });
            if resp.hovered() {
                let anchor = hover.unwrap_or(rect.center());
                if zoom_factor != 1.0 {
                    let target = self.zoom + zoom_factor.log2().clamp(-2.0, 2.0);
                    self.zoom_to(anchor, target, &rect);
                } else if scroll != 0.0 {
                    // Smoothed line deltas: ~+0.2 zoom per wheel notch.
                    self.zoom_to(anchor, self.zoom + scroll * 0.005, &rect);
                }
            }
            if resp.dragged() {
                // Grab-and-drag: content follows the cursor.
                let delta = resp.drag_delta();
                let world = world_px(self.zoom);
                self.center_lng = merc_lng(merc_x(self.center_lng, world) - delta.x as f64, world);
                self.center_lat = merc_lat(merc_y(self.center_lat, world) - delta.y as f64, world);
                self.status = "pan".to_string();
            }
        });
    }
}

/// Fit visible features into a WxH view. Shared by the canvas and the
/// headless BMP preview so pixels and preview agree. None when empty.
/// With `only`, frames just that layer (Zoom to layer).
fn fit_view(
    features: &[store::StoredFeature],
    layers: &[LayerRow],
    width: f32,
    height: f32,
    only: Option<&str>,
) -> Option<(f64, f64, f32)> {
    let visible = |id: &str| {
        only.is_none_or(|o| o == id)
            && layers
                .iter()
                .find(|l| l.id == id)
                .map(|l| l.visible)
                .unwrap_or(false)
    };
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for feat in features {
        if !visible(feat.layer_id.as_str()) {
            continue;
        }
        // World01 (zoom-0 px / 256): zoom-independent, no overflow.
        let x = (feat.lng + 180.0) / 360.0;
        let r = feat.lat.clamp(-85.05112878, 85.05112878).to_radians();
        let y = (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if min_x.is_infinite() {
        return None;
    }
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    // 20% padding so edge points never sit under the frame.
    let span_x = ((max_x - min_x) * 1.2).max(1e-9);
    let span_y = ((max_y - min_y) * 1.2).max(1e-9);
    let zoom = (width / (span_x * 256.0) as f32).min(height / (span_y * 256.0) as f32);
    let zoom = zoom.log2().clamp(2.0, 19.0);
    // Back to lng/lat.
    let clng = cx * 360.0 - 180.0;
    let t = std::f64::consts::PI * (1.0 - 2.0 * cy);
    let clat = t.sinh().atan().to_degrees();
    Some((clng, clat, zoom))
}

/// Headless BMP preview (24-bit, hand-rolled): same `project` math as the
/// canvas, XYZ tiles under cyan points. Lets a windowless session see the
/// map. `with_tiles=false` keeps unit tests offline; smoke passes true.
fn render_preview(
    features: &[store::StoredFeature],
    layers: &[LayerRow],
    width: u32,
    height: u32,
    with_tiles: bool,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("preview size must be 1..4096".to_string());
    }
    let rect = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::Vec2::new(width as f32, height as f32),
    );
    let (clng, clat, zoom) = fit_view(features, layers, width as f32, height as f32, None)
        .unwrap_or((16.3738, 48.2082, 15.0));
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
    // Pixel writer: explicit buffer param so helpers stay borrow-free.
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
    let blend_over = |px: &mut [u8], x: i32, y: i32, b: u8, g: u8, r: u8| {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let i = (((height as i32 - 1 - y) as u32 * width + x as u32) * 3) as usize;
        if i + 2 < px.len() {
            // 35% foreground over background, integer math, no overflow.
            px[i] = ((b as u32 * 35 + px[i] as u32 * 65) / 100) as u8;
            px[i + 1] = ((g as u32 * 35 + px[i + 1] as u32 * 65) / 100) as u8;
            px[i + 2] = ((r as u32 * 35 + px[i + 2] as u32 * 65) / 100) as u8;
        }
    };
    // DDA segment rasterizer over an explicit buffer.
    let seg = |px: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, b: u8, g: u8, r: u8| {
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            plot(
                px,
                (x0 as f32 + (x1 - x0) as f32 * t).round() as i32,
                (y0 as f32 + (y1 - y0) as f32 * t).round() as i32,
                b,
                g,
                r,
            );
        }
    };
    // Half-space triangle fill over an explicit buffer (either winding).
    let fill_tri = |px: &mut [u8],
                    ax: i32,
                    ay: i32,
                    bx: i32,
                    by: i32,
                    cx: i32,
                    cy: i32,
                    b: u8,
                    g: u8,
                    r: u8| {
        let edge = |px0: i32, py0: i32, qx: i32, qy: i32, rx: i32, ry: i32| {
            (rx - px0) * (qy - py0) - (ry - py0) * (qx - px0)
        };
        let area = edge(ax, ay, bx, by, cx, cy);
        if area == 0 {
            return;
        }
        let min_x = ax.min(bx).min(cx).max(0);
        let max_x = (ax.max(bx).max(cx) + 1).min(width as i32 - 1);
        let min_y = ay.min(by).min(cy).max(0);
        let max_y = (ay.max(by).max(cy) + 1).min(height as i32 - 1);
        for py in min_y..=max_y {
            for qx in min_x..=max_x {
                let w0 = edge(bx, by, cx, cy, qx, py);
                let w1 = edge(cx, cy, ax, ay, qx, py);
                let w2 = edge(ax, ay, bx, by, qx, py);
                let inside = if area > 0 {
                    w0 >= 0 && w1 >= 0 && w2 >= 0
                } else {
                    w0 <= 0 && w1 <= 0 && w2 <= 0
                };
                if inside {
                    blend_over(px, qx, py, b, g, r);
                }
            }
        }
    };
    // Tiles first (best-effort: any failure leaves dark bg, never aborts).
    // Pull-model: iterate destination pixels inside each tile's screen
    // rect and sample the source (nearest). Push-model (one plot per
    // source pixel) leaves subpixel gaps at non-integer zooms.
    if with_tiles {
        let tile_z = (zoom.floor() as i64).clamp(0, 19) as u8;
        let (wlng, nlat) = unproject(rect.min, clng, clat, zoom, &rect);
        let (elng, slat) = unproject(rect.max, clng, clat, zoom, &rect);
        if let Ok(list) = tiles::bbox_to_tiles(wlng, slat, elng, nlat, tile_z) {
            for t in list {
                let png = match tiles::fetch_tile_png(t) {
                    Ok(png) => png,
                    Err(_) => continue,
                };
                let (tw, th, rgba) = match tiles::decode_png_rgba(&png) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let (tmin_lng, tmin_lat, tmax_lng, tmax_lat) = tiles::tile_bounds(t);
                let a = project(tmin_lng, tmax_lat, clng, clat, zoom, &rect);
                let b = project(tmax_lng, tmin_lat, clng, clat, zoom, &rect);
                let x0 = a.x.floor() as i32;
                let x1 = b.x.ceil() as i32;
                let y0 = a.y.floor() as i32;
                let y1 = b.y.ceil() as i32;
                let denom_x = (b.x - a.x).max(1.0);
                let denom_y = (b.y - a.y).max(1.0);
                for dy in y0..=y1 {
                    for dx in x0..=x1 {
                        let sxx = (((dx as f32 - a.x) / denom_x * tw as f32) as i64)
                            .clamp(0, tw as i64 - 1) as u32;
                        let syy = (((dy as f32 - a.y) / denom_y * th as f32) as i64)
                            .clamp(0, th as i64 - 1) as u32;
                        let si = ((syy * tw + sxx) * 4) as usize;
                        if si + 2 < rgba.len() {
                            plot(&mut px, dx, dy, rgba[si + 2], rgba[si + 1], rgba[si]);
                        }
                    }
                }
            }
        }
    }
    for feat in features {
        let vis = layers
            .iter()
            .find(|l| l.id == feat.layer_id)
            .map(|l| l.visible)
            .unwrap_or(false);
        if !vis {
            continue;
        }
        // Same palette as the canvas; DDA outlines, earcut fills.
        let c = layer_color(feat.layer_id.as_str());
        let (cr, cg, cb) = (c.r(), c.g(), c.b());
        match parse_geom(feat) {
            Some(Geom::Poly(ring)) => {
                // Ear-clipped fill first, outline on top.
                let tris = triangulate::earcut_tris(&ring);
                for t in tris.as_chunks::<3>().0 {
                    let [ia, ib, ic] = *t;
                    if let (Some(a), Some(b), Some(c)) = (ring.get(ia), ring.get(ib), ring.get(ic))
                    {
                        let pa = project(a.0, a.1, clng, clat, zoom, &rect);
                        let pb = project(b.0, b.1, clng, clat, zoom, &rect);
                        let pc = project(c.0, c.1, clng, clat, zoom, &rect);
                        fill_tri(
                            &mut px,
                            pa.x.round() as i32,
                            pa.y.round() as i32,
                            pb.x.round() as i32,
                            pb.y.round() as i32,
                            pc.x.round() as i32,
                            pc.y.round() as i32,
                            cb,
                            cg,
                            cr,
                        );
                    }
                }
                let pts: Vec<(i32, i32)> = ring
                    .iter()
                    .map(|(lng, lat)| {
                        let p = project(*lng, *lat, clng, clat, zoom, &rect);
                        (p.x.round() as i32, p.y.round() as i32)
                    })
                    .collect();
                let n = pts.len();
                for i in 0..n {
                    let (x0, y0) = pts[i % n];
                    let (x1, y1) = pts[(i + 1) % n];
                    seg(&mut px, x0, y0, x1, y1, cb, cg, cr);
                }
            }
            Some(Geom::Line(line)) => {
                let pts: Vec<(i32, i32)> = line
                    .iter()
                    .map(|(lng, lat)| {
                        let p = project(*lng, *lat, clng, clat, zoom, &rect);
                        (p.x.round() as i32, p.y.round() as i32)
                    })
                    .collect();
                let n = pts.len();
                for i in 0..n.saturating_sub(1) {
                    seg(
                        &mut px,
                        pts[i].0,
                        pts[i].1,
                        pts[i + 1].0,
                        pts[i + 1].1,
                        cb,
                        cg,
                        cr,
                    );
                }
            }
            None => {
                let pos = project(feat.lng, feat.lat, clng, clat, zoom, &rect);
                let cx = pos.x.round() as i32;
                let cy = pos.y.round() as i32;
                // 4px disk.
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        if dx * dx + dy * dy <= 6 {
                            plot(&mut px, cx + dx, cy + dy, cb, cg, cr);
                        }
                    }
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
    let app = QgisCraftApp::open_with_seed(None);
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
    match render_preview(&app.features, &app.layers, 1280, 860, true) {
        Ok(bytes) => match std::fs::write(&preview_path, &bytes) {
            Ok(()) => println!("preview: {}", preview_path.to_string_lossy()),
            Err(e) => println!("preview write failed: {e}"),
        },
        Err(e) => println!("preview render failed: {e}"),
    }
}

fn control_port_from_args() -> Option<u16> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--control" {
            if let Some(port) = args.next() {
                if let Ok(port) = port.parse::<u16>() {
                    if port != 0 {
                        return Some(port);
                    }
                }
            }
            return None;
        }
    }
    std::env::var("QGISCRAFT_CONTROL_PORT")
        .ok()
        .and_then(|raw| raw.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
}

fn main() {
    if std::env::args().any(|arg| arg == "--smoke") {
        return smoke();
    }
    let control_port = control_port_from_args();
    let (control_tx, control_rx) = std::sync::mpsc::channel();
    if let Some(port) = control_port {
        match std::net::TcpListener::bind(format!("127.0.0.1:{port}")) {
            Ok(listener) => {
                eprintln!(
                    "qgiscraft control listening on 127.0.0.1:{port} (no auth: local use only)"
                );
                std::thread::spawn(move || control::serve(listener, control_tx));
            }
            Err(e) => eprintln!("control bind failed (continuing without): {e}"),
        }
    }
    let control_rx = control_port.map(|_| control_rx);
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
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(high_contrast_dark());
            Ok(Box::new(QgisCraftApp::open_with_seed(control_rx)))
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
    fn zoom_doubles_scale_per_level() {
        let rect = test_rect();
        let a = project(16.3838, 48.2082, 16.3738, 48.2082, 15.0, &rect);
        let b = project(16.3838, 48.2082, 16.3738, 48.2082, 16.0, &rect);
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
                geom_json: None,
            },
            store::StoredFeature {
                layer_id: "rx".to_string(),
                name: "B".to_string(),
                kind: "node".to_string(),
                lng: 16.40,
                lat: 48.25,
                geom_json: None,
            },
        ]
    }

    #[test]
    fn fit_view_centers_data() {
        let fit = fit_view(&test_features(), &test_layers(), 800.0, 600.0, None);
        assert!(fit.is_some());
        if let Some((clng, clat, zoom)) = fit {
            assert!((clng - 16.35).abs() < 1e-6);
            assert!((clat - 48.225).abs() < 1e-3);
            assert!((2.0..=19.0).contains(&zoom));
        }
    }

    #[test]
    fn fit_view_empty_is_none() {
        assert!(fit_view(&[], &test_layers(), 800.0, 600.0, None).is_none());
    }

    #[test]
    fn fit_view_only_unknown_layer_is_none() {
        assert!(fit_view(&test_features(), &test_layers(), 800.0, 600.0, Some("nope")).is_none());
    }

    #[test]
    fn render_preview_is_bmp_and_rejects_hostile_size() {
        assert!(render_preview(&test_features(), &test_layers(), 0, 600, false).is_err());
        assert!(render_preview(&test_features(), &test_layers(), 5000, 600, false).is_err());
        let bmp = render_preview(&test_features(), &test_layers(), 320, 200, false);
        assert!(bmp.is_ok());
        if let Ok(bytes) = bmp {
            assert!(bytes.len() > 54);
            assert_eq!(&bytes[0..2], b"BM");
        }
    }

    #[test]
    fn polygon_fill_paints_interior() {
        // One square polygon, no tiles: the view center must carry the
        // blended fill color (35% layer color over near-black bg).
        let feat = geom_feature(
            r#"{"type":"Polygon","coordinates":[[[16.3,48.2],[16.4,48.2],[16.4,48.25],[16.3,48.2]]]}"#,
        );
        let bmp = render_preview(&[feat], &test_layers(), 320, 200, false);
        assert!(bmp.is_ok());
        if let Ok(bytes) = bmp {
            let w = 320_usize;
            let h = 200_usize;
            let stride = w * 3; // 960, already multiple of 4
            let (cx, cy) = (160_usize, 100_usize);
            let i = 54 + (h - 1 - cy) * stride + cx * 3;
            let c = layer_color("rx");
            let exp_b = (u32::from(c.b()) * 35 + 7 * 65) / 100;
            let exp_g = (u32::from(c.g()) * 35 + 5 * 65) / 100;
            let exp_r = (u32::from(c.r()) * 35 + 5 * 65) / 100;
            assert_eq!(bytes[i] as u32, exp_b, "fill blue channel");
            assert_eq!(bytes[i + 1] as u32, exp_g, "fill green channel");
            assert_eq!(bytes[i + 2] as u32, exp_r, "fill red channel");
        }
    }

    fn geom_feature(geom: &str) -> store::StoredFeature {
        store::StoredFeature {
            layer_id: "rx".to_string(),
            name: "G".to_string(),
            kind: "way".to_string(),
            lng: 16.35,
            lat: 48.225,
            geom_json: Some(geom.to_string()),
        }
    }

    #[test]
    fn parse_geom_accepts_line_and_polygon() {
        let line =
            geom_feature(r#"{"type":"LineString","coordinates":[[16.3,48.2],[16.4,48.25]]}"#);
        assert!(matches!(parse_geom(&line), Some(Geom::Line(_))));
        let poly = geom_feature(
            r#"{"type":"Polygon","coordinates":[[[16.3,48.2],[16.4,48.2],[16.4,48.25],[16.3,48.2]]]}"#,
        );
        assert!(matches!(parse_geom(&poly), Some(Geom::Poly(_))));
    }

    #[test]
    fn parse_geom_rejects_garbage() {
        let no_geom = store::StoredFeature {
            geom_json: None,
            ..test_features()[0].clone()
        };
        assert!(parse_geom(&no_geom).is_none());
        let bad_type = geom_feature(r#"{"type":"MultiPoint","coordinates":[[16.3,48.2]]}"#);
        assert!(parse_geom(&bad_type).is_none());
        let bad_json = geom_feature("{not json");
        assert!(parse_geom(&bad_json).is_none());
        let short_line = geom_feature(r#"{"type":"LineString","coordinates":[[16.3,48.2]]}"#);
        assert!(parse_geom(&short_line).is_none());
        let oob =
            geom_feature(r#"{"type":"LineString","coordinates":[[999.0,48.2],[16.4,48.25]]}"#);
        // One bad point skipped, one good point left -> too short.
        assert!(parse_geom(&oob).is_none());
    }

    #[test]
    fn layer_color_is_deterministic() {
        assert_eq!(layer_color("a"), layer_color("a"));
    }
}
