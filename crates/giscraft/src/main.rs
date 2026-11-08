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
use giscraft_core::control;
use giscraft_core::geo;
use giscraft_core::import;
use giscraft_core::tiles;
use giscraft_core::triangulate;
use giscraft_core::wkt;
use giscraft_store as store;
use std::collections::HashMap;

/// Modal dialog state. Only one open at a time; Cancel closes without doing.
enum Dialog {
    None,
    XyzTile {
        name: String,
        url: String,
    },
    Scratch {
        name: String,
        geom: usize,
    },
    OsmDownload {
        key: String,
        value: String,
        use_view: bool,
    },
    Geocode {
        query: String,
        limit: String,
    },
    Wkt {
        text: String,
        layer_id: String,
    },
    ZoomTo {
        text: String,
    },
    Grid {
        south: String,
        west: String,
        north: String,
        east: String,
        cell_size_m: String,
        hex: bool,
        layer_id: String,
    },
    Nearest {
        source: String,
        target: String,
        output: String,
    },
    Geodesic {
        shape: String,
        lng: String,
        lat: String,
        radius_m: String,
        major_m: String,
        minor_m: String,
        bearing: String,
        lng2: String,
        lat2: String,
        layer_id: String,
    },
    Doppler {
        capture: String,
        layer_id: String,
    },
    Buffer {
        radius_m: String,
    },
}

/// One finished background download, ready to ingest on the UI thread.
struct IngestBatch {
    layer_id: String,
    layer_name: String,
    features: Vec<store::StoredFeature>,
}

/// Resolve the active tile template: first visible raster layer whose
/// source is a *usable* `xyz:<template>` wins; anything else (legacy stubs
/// like `xyz:osm`, user typos) is skipped so it can never shadow the
/// default basemap with a broken URL.
fn resolve_tile_template(layers: &[LayerRow]) -> String {
    for l in layers {
        if l.visible && l.geom_type == "raster" {
            if let Some(t) = l.source.strip_prefix("xyz:") {
                if tiles::is_usable_template(t) {
                    return t.to_string();
                }
            }
        }
    }
    tiles::tile_url_template()
}

const GEOM_KINDS: [&str; 3] = ["point", "line", "polygon"];

const XYZ_PRESETS: [(&str, &str); 4] = [
    (
        "xyz-osm",
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
    ),
    (
        "xyz-hot",
        "https://tile.openstreetmap.fr/hot/{z}/{x}/{y}.png",
    ),
    (
        "xyz-carto-dark",
        "https://basemaps.cartocdn.com/dark_all/{z}/{x}/{y}.png",
    ),
    (
        "xyz-esri-imagery",
        "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
    ),
];

struct LayerRow {
    id: String,
    name: String,
    source: String,
    geom_type: String,
    visible: bool,
    color: String,
    features: u64,
}

struct GisCraftApp {
    db_path: String,
    locale: giscraft_core::i18n::AppLocale,
    layers: Vec<LayerRow>,
    features: Vec<store::StoredFeature>,
    selected: Option<String>,
    status: String,
    log: Vec<String>,
    zoom: f32,
    center_lng: f64,
    center_lat: f64,
    fit_requested: bool,
    fit_only: Option<String>,
    confirm_new: bool,
    show_about: bool,
    show_help: bool,
    show_settings: bool,
    dialog: Dialog,
    toolbox_query: String,
    pending: Option<std::sync::mpsc::Receiver<Result<IngestBatch, String>>>,
    tile_template: String,
    textures: HashMap<String, egui::TextureHandle>,
    control_rx: Option<std::sync::mpsc::Receiver<control::ControlRequest>>,
    last_rect: Option<egui::Rect>,
}

/// Screen projection, shared with the web preview via `tiles`.
/// Thin egui wrappers: the math lives in core so both renderers agree.
fn project(
    lng: f64,
    lat: f64,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> egui::Pos2 {
    let c = rect.center();
    let (x, y) = tiles::project(lng, lat, center_lng, center_lat, zoom, c.x, c.y);
    egui::Pos2::new(x, y)
}

fn unproject(
    pos: egui::Pos2,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    rect: &egui::Rect,
) -> (f64, f64) {
    let c = rect.center();
    tiles::unproject(pos.x, pos.y, center_lng, center_lat, zoom, c.x, c.y)
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

impl GisCraftApp {
    fn open_with_seed(
        control_rx: Option<std::sync::mpsc::Receiver<control::ControlRequest>>,
    ) -> Self {
        let db_path = store::default_db_path().to_string_lossy().to_string();
        let mut app = Self {
            db_path,
            locale: giscraft_core::i18n::AppLocale::default(),
            layers: Vec::new(),
            features: Vec::new(),
            selected: None,
            status: "render on".to_string(),
            log: Vec::new(),
            zoom: 15.0,
            center_lng: 16.3738,
            center_lat: 48.2082,
            fit_requested: true,
            fit_only: None,
            confirm_new: false,
            show_about: false,
            show_help: false,
            show_settings: false,
            dialog: Dialog::None,
            toolbox_query: String::new(),
            pending: None,
            tile_template: tiles::tile_url_template(),
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
                        Some(format!("xyz:{}", tiles::tile_url_template())),
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
                                    source: l.source,
                                    geom_type: l.geom_type,
                                    visible: l.visible,
                                    color: l.color,
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

    /// Canvas color: stored #rrggbb wins, else the deterministic palette.
    fn layer_draw_color(&self, layer_id: &str) -> egui::Color32 {
        layer_draw_color(&self.layers, layer_id)
    }
}

/// Filesystem/URL-safe layer id from a human name.
fn slug(name: &str) -> String {
    let mut out: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "layer".to_string()
    } else {
        out
    }
}

/// Deterministic per-layer color from a fixed high-contrast palette.
/// Same layer id always maps to the same color (canvas + preview agree).
fn layer_color(id: &str) -> egui::Color32 {
    let (r, g, b) = tiles::layer_color(id);
    egui::Color32::from_rgb(r, g, b)
}

/// Stored #rrggbb wins; empty falls back to the palette. Shared by the
/// canvas (via the method above) and the headless BMP renderer.
fn layer_draw_color(layers: &[LayerRow], id: &str) -> egui::Color32 {
    if let Some(l) = layers.iter().find(|l| l.id == id) {
        if let Ok(Some((r, g, b))) = tiles::parse_layer_color(&l.color) {
            return egui::Color32::from_rgb(r, g, b);
        }
    }
    layer_color(id)
}

/// True when (lng, lat) is within `tol` degrees of a polyline
/// (or closed ring when `closed`). Segment math lives in core.
fn near_polyline(lng: f64, lat: f64, pts: &[(f64, f64)], closed: bool, tol: f64) -> bool {
    if pts.len() < 2 {
        return false;
    }
    let n = pts.len();
    let end = if closed { n } else { n - 1 };
    for i in 0..end {
        let a = match pts.get(i) {
            Some(&p) => p,
            None => continue,
        };
        let b = match pts.get((i + 1) % n) {
            Some(&p) => p,
            None => continue,
        };
        if tiles::dist_to_segment_deg(lng, lat, a.0, a.1, b.0, b.1) <= tol {
            return true;
        }
    }
    false
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

/// Hit-test result: what the cursor points at (module scope by necessity:
/// structs and impl blocks cannot nest inside another impl).
struct HitInfo {
    layer_id: String,
    name: String,
    kind: String,
    lng: f64,
    lat: f64,
}

/// Frame visible features with padding. No-op when nothing is visible.
impl GisCraftApp {
    /// Identify the topmost visible feature under a screen point.
    /// Points: within 12px (cos-corrected). Lines: within 12px of any
    /// segment. Polygons: inside, or within 12px of an edge.
    /// (The zoom-to-cursor helper `zoom_to` follows in this same impl.)
    fn pick_at(&self, pos: egui::Pos2, rect: &egui::Rect) -> Option<HitInfo> {
        if !rect.contains(pos) {
            return None;
        }
        let (glng, glat) = unproject(pos, self.center_lng, self.center_lat, self.zoom, rect);
        if !glng.is_finite() || !glat.is_finite() {
            return None;
        }
        let cos_lat = self.center_lat.to_radians().cos().max(0.1);
        let tol = 12.0 * tiles::meters_per_pixel(self.center_lat, self.zoom) / 111320.0 / cos_lat;
        if tol <= 0.0 || !tol.is_finite() {
            return None;
        }
        let hit = |f: &store::StoredFeature| HitInfo {
            layer_id: f.layer_id.clone(),
            name: f.name.clone(),
            kind: f.kind.clone(),
            lng: f.lng,
            lat: f.lat,
        };
        for f in self.features.iter().rev() {
            if !self.is_visible(f.layer_id.as_str()) {
                continue;
            }
            match parse_geom(f) {
                Some(Geom::Poly(ring)) => {
                    if ring.len() >= 3 && tiles::point_in_ring(glng, glat, &ring) {
                        return Some(hit(f));
                    }
                    if near_polyline(glng, glat, &ring, true, tol) {
                        return Some(hit(f));
                    }
                }
                Some(Geom::Line(line)) => {
                    if near_polyline(glng, glat, &line, false, tol) {
                        return Some(hit(f));
                    }
                }
                None => {
                    let dx = (f.lng - glng) * cos_lat;
                    let dy = f.lat - glat;
                    if dx * dx + dy * dy <= tol * tol {
                        return Some(hit(f));
                    }
                }
            }
        }
        None
    }

    fn zoom_to(&mut self, anchor: egui::Pos2, target: f32, rect: &egui::Rect) {
        let (glng, glat) = unproject(anchor, self.center_lng, self.center_lat, self.zoom, rect);
        self.zoom = target.clamp(2.0, 19.0);
        let c = rect.center();
        let (nclng, nclat) = tiles::recenter(glng, glat, anchor.x, anchor.y, c.x, c.y, self.zoom);
        self.center_lng = nclng;
        self.center_lat = nclat;
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
                        Some(format!("xyz:{}", tiles::tile_url_template())),
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

    /// Plugins > Zoom to coordinates: center the canvas on parsed lat,lng.
    fn zoom_to_coords(&mut self, text: &str) {
        match geo::parse_latlng(text) {
            Ok(p) => {
                self.center_lng = p.lng;
                self.center_lat = p.lat;
                self.zoom = 15.0;
                self.status = "zoom".to_string();
                self.push_log(format!("zoom to {}, {}", p.lat, p.lng));
            }
            Err(e) => self.push_log(format!("coords failed: {e}")),
        }
    }

    /// Plugins > Show WKT: parse and persist as a layer, then fit it.
    fn submit_wkt(&mut self, text: &str, layer_id: &str) {
        if layer_id.is_empty() || layer_id.chars().count() > 64 {
            self.push_log("wkt failed: layer id must be 1..64 chars".to_string());
            return;
        }
        let feats = match wkt::parse_wkt(text) {
            Ok(f) => f,
            Err(e) => {
                self.push_log(format!("wkt failed: {e}"));
                return;
            }
        };
        if feats.is_empty() {
            self.push_log("wkt: empty geometry, nothing added".to_string());
            return;
        }
        let mut geom_type = "point";
        for f in &feats {
            if f.geojson.contains("\"Polygon\"") {
                geom_type = "polygon";
                break;
            }
            if f.geojson.contains("\"LineString\"") {
                geom_type = "line";
            }
        }
        let rows: Vec<store::StoredFeature> = feats
            .iter()
            .map(|f| store::StoredFeature {
                layer_id: layer_id.to_string(),
                name: f.name.clone(),
                kind: f.kind.clone(),
                lng: f.lng,
                lat: f.lat,
                geom_json: Some(f.geojson.clone()),
            })
            .collect();
        let n = rows.len();
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: layer_id.to_string(),
                    name: layer_id.to_string(),
                    source: "wkt".to_string(),
                    geom_type: geom_type.to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer)
                    .and_then(|()| store::replace_features(&conn, layer_id, &rows).map(|_| ()))
                {
                    Ok(()) => format!("wkt: {n} features into {layer_id}"),
                    Err(e) => format!("wkt ingest failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(layer_id.to_string());
        self.refresh();
        self.fit_only = Some(layer_id.to_string());
        self.fit_requested = true;
        self.push_log(outcome);
    }

    /// Processing toolbox > Create grid: build rect/hex cells, persist, fit.
    fn submit_grid(&mut self, args: (String, String, String, String, String, bool, String)) {
        let (south, west, north, east, cell_size_m, hex, layer_id) = args;
        if layer_id.is_empty() || layer_id.chars().count() > 64 {
            self.push_log("grid failed: layer id must be 1..64 chars".to_string());
            return;
        }
        let mut parse = |t: &str, label: &str| -> Option<f64> {
            match t.trim().parse::<f64>() {
                Ok(v) if v.is_finite() => Some(v),
                _ => {
                    self.push_log(format!("grid failed: bad {label}"));
                    None
                }
            }
        };
        let (Some(s), Some(w), Some(n), Some(e), Some(cell)) = (
            parse(&south, "south"),
            parse(&west, "west"),
            parse(&north, "north"),
            parse(&east, "east"),
            parse(&cell_size_m, "cell size"),
        ) else {
            return;
        };
        let feats = match giscraft_core::processing::grid_features(s, w, n, e, cell, hex) {
            Ok(f) => f,
            Err(e) => {
                self.push_log(format!("grid failed: {e}"));
                return;
            }
        };
        let rows: Vec<store::StoredFeature> = feats
            .iter()
            .map(|f| store::StoredFeature {
                layer_id: layer_id.clone(),
                name: f.name.clone(),
                kind: f.kind.clone(),
                lng: f.lng,
                lat: f.lat,
                geom_json: f.geom_json.clone(),
            })
            .collect();
        let n = rows.len();
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: layer_id.clone(),
                    name: layer_id.clone(),
                    source: "processing:grid".to_string(),
                    geom_type: "polygon".to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer)
                    .and_then(|()| store::replace_features(&conn, &layer_id, &rows).map(|_| ()))
                {
                    Ok(()) => format!("grid: {n} cells into {layer_id}"),
                    Err(e) => format!("grid ingest failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(layer_id.clone());
        self.refresh();
        self.fit_only = Some(layer_id.clone());
        self.fit_requested = true;
        self.push_log(outcome);
    }

    /// Processing toolbox > Nearest neighbor join: hub lines, persist, fit.
    fn submit_nearest(&mut self, args: (String, String, String)) {
        let (source, target, output) = args;
        if source.trim().is_empty() || target.trim().is_empty() {
            self.push_log("nearest failed: source and target layers required".to_string());
            return;
        }
        let out_id = if output.trim().is_empty() {
            format!("{source}-nearest")
        } else {
            output.trim().to_string()
        };
        let outcome = match store::open_default() {
            Ok(conn) => {
                let load = |id: &str| -> Result<Vec<giscraft_core::import::RawFeature>, String> {
                    match store::list_features(&conn, Some(id)) {
                        Ok(rows) => Ok(rows
                            .into_iter()
                            .map(|f| giscraft_core::import::RawFeature {
                                name: f.name,
                                kind: f.kind,
                                lng: f.lng,
                                lat: f.lat,
                                geom_json: f.geom_json,
                            })
                            .collect()),
                        Err(e) => Err(e.to_string()),
                    }
                };
                let joined = load(source.trim()).and_then(|s| load(target.trim()).map(|t| (s, t)));
                match joined {
                    Err(e) => format!("nearest failed: {e}"),
                    Ok((s, t)) => match giscraft_core::processing::nearest_join(&s, &t) {
                        Err(e) => format!("nearest failed: {e}"),
                        Ok(lines) => {
                            let rows: Vec<store::StoredFeature> = lines
                                .iter()
                                .map(|f| store::StoredFeature {
                                    layer_id: out_id.clone(),
                                    name: f.name.clone(),
                                    kind: "way".to_string(),
                                    lng: f.lng,
                                    lat: f.lat,
                                    geom_json: f.geom_json.clone(),
                                })
                                .collect();
                            let n = rows.len();
                            let layer = store::StoredLayer {
                                id: out_id.clone(),
                                name: out_id.clone(),
                                source: "processing:nearest".to_string(),
                                geom_type: "line".to_string(),
                                crs: "EPSG:4326".to_string(),
                                visible: true,
                                color: String::new(),
                            };
                            match store::add_layer(&conn, &layer).and_then(|()| {
                                store::replace_features(&conn, &out_id, &rows).map(|_| ())
                            }) {
                                Ok(()) => format!("nearest: {n} hub lines into {out_id}"),
                                Err(e) => format!("nearest ingest failed: {e}"),
                            }
                        }
                    },
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(out_id.clone());
        self.refresh();
        self.fit_only = Some(out_id.clone());
        self.fit_requested = true;
        self.push_log(outcome);
    }

    /// Processing toolbox > Geodesic shape: true-distance circle/ellipse/line.
    fn submit_geodesic(
        &mut self,
        args: (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        ),
    ) {
        let (shape, lng, lat, radius_m, major_m, minor_m, bearing, lng2, lat2, layer_id) = args;
        let shape = shape.trim().to_ascii_lowercase();
        if shape != "circle" && shape != "ellipse" && shape != "line" {
            self.push_log("geodesic failed: shape must be circle, ellipse or line".to_string());
            return;
        }
        let mut num = |t: &str, label: &str| -> Option<f64> {
            match t.trim().parse::<f64>() {
                Ok(v) if v.is_finite() => Some(v),
                _ => {
                    self.push_log(format!("geodesic failed: bad {label}"));
                    None
                }
            }
        };
        let (Some(x), Some(y)) = (num(&lng, "center lng"), num(&lat, "center lat")) else {
            return;
        };
        let ring = if shape == "circle" {
            let Some(r) = num(&radius_m, "radius") else {
                return;
            };
            match giscraft_core::processing::geodesic_circle(x, y, r, 64) {
                Ok(ring) => ring,
                Err(e) => {
                    self.push_log(format!("geodesic failed: {e}"));
                    return;
                }
            }
        } else if shape == "ellipse" {
            let (Some(a), Some(b), Some(br)) = (
                num(&major_m, "major"),
                num(&minor_m, "minor"),
                num(&bearing, "bearing"),
            ) else {
                return;
            };
            match giscraft_core::processing::geodesic_ellipse(x, y, a, b, br, 64) {
                Ok(ring) => ring,
                Err(e) => {
                    self.push_log(format!("geodesic failed: {e}"));
                    return;
                }
            }
        } else {
            let (Some(x2), Some(y2)) = (num(&lng2, "end lng"), num(&lat2, "end lat")) else {
                return;
            };
            match giscraft_core::processing::xy_to_line(x, y, x2, y2) {
                Ok(pts) => pts,
                Err(e) => {
                    self.push_log(format!("geodesic failed: {e}"));
                    return;
                }
            }
        };
        let geom_kind = if shape == "line" { "line" } else { "polygon" };
        let mut closed = ring.clone();
        if shape != "line" {
            if let Some(first) = ring.first().copied() {
                if closed.last() != Some(&first) {
                    closed.push(first);
                }
            }
        }
        let inner: Vec<String> = closed.iter().map(|(a, b)| format!("[{a},{b}]")).collect();
        let coords = if shape == "line" {
            format!("[{}]", inner.join(","))
        } else {
            format!("[[{}]]", inner.join(","))
        };
        let geojson = if geom_kind == "line" {
            format!("{{\"type\":\"LineString\",\"coordinates\":{coords}}}")
        } else {
            format!("{{\"type\":\"Polygon\",\"coordinates\":{coords}}}")
        };
        let rows = vec![store::StoredFeature {
            layer_id: layer_id.clone(),
            name: format!("{shape}-1"),
            kind: "way".to_string(),
            lng: x,
            lat: y,
            geom_json: Some(geojson),
        }];
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: layer_id.clone(),
                    name: layer_id.clone(),
                    source: "processing:geodesic".to_string(),
                    geom_type: geom_kind.to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer)
                    .and_then(|()| store::replace_features(&conn, &layer_id, &rows).map(|_| ()))
                {
                    Ok(()) => format!("geodesic: {shape} into {layer_id}"),
                    Err(e) => format!("geodesic ingest failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(layer_id.clone());
        self.refresh();
        self.fit_only = Some(layer_id.clone());
        self.fit_requested = true;
        self.push_log(outcome);
    }

    /// Processing toolbox > Doppler locate: rank candidates, persist top hits.
    /// Bbox comes from the current canvas; step auto-scales to ~2500 cells.
    fn submit_doppler(&mut self, path: &str, layer_id: &str) {
        if layer_id.is_empty() || layer_id.chars().count() > 64 {
            self.push_log("doppler failed: layer id must be 1..64 chars".to_string());
            return;
        }
        let text = match std::fs::read_to_string(path.trim()) {
            Ok(t) => t,
            Err(e) => {
                self.push_log(format!("doppler failed: read failed: {e}"));
                return;
            }
        };
        let cap = match giscraft_core::doppler::parse_capture(&text) {
            Ok(c) => c,
            Err(e) => {
                self.push_log(format!("doppler failed: {e}"));
                return;
            }
        };
        let bbox = match self.view_bbox() {
            Some(b) => b,
            None => {
                self.push_log("doppler failed: no canvas extent yet".to_string());
                return;
            }
        };
        let span = (bbox.max_lng - bbox.min_lng)
            .max(bbox.max_lat - bbox.min_lat)
            .max(0.0001);
        let step = (span / 50.0).clamp(0.0005, 1.0);
        let ranked = match giscraft_core::doppler::locate(
            &giscraft_core::doppler::LocateParams {
                south: bbox.min_lat,
                west: bbox.min_lng,
                north: bbox.max_lat,
                east: bbox.max_lng,
                step_deg: step,
                top_n: 10,
                terrain_m: 200.0,
            },
            &cap,
        ) {
            Ok(r) => r,
            Err(e) => {
                self.push_log(format!("doppler failed: {e}"));
                return;
            }
        };
        let rows: Vec<store::StoredFeature> = ranked
            .iter()
            .enumerate()
            .map(|(i, p)| store::StoredFeature {
                layer_id: layer_id.to_string(),
                name: format!("cand-{} ({:.2})", i + 1, p.score),
                kind: "point".to_string(),
                lng: p.lng,
                lat: p.lat,
                geom_json: None,
            })
            .collect();
        let n = rows.len();
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: layer_id.to_string(),
                    name: layer_id.to_string(),
                    source: "doppler".to_string(),
                    geom_type: "point".to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer)
                    .and_then(|()| store::replace_features(&conn, layer_id, &rows).map(|_| ()))
                {
                    Ok(()) => {
                        format!("doppler: top {n} into {layer_id} (terrain 200m, step {step:.4})")
                    }
                    Err(e) => format!("doppler ingest failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(layer_id.to_string());
        self.refresh();
        self.fit_only = Some(layer_id.to_string());
        self.fit_requested = true;
        self.push_log(outcome);
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

    /// Persist one batch of parsed/imported features under `layer_id`
    /// (replace semantics). Shared by file import and background downloads.
    fn persist_batch(
        &mut self,
        layer_id: &str,
        layer_name: &str,
        source: &str,
        geom_type: &str,
        feats: &[import::RawFeature],
    ) {
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: layer_id.to_string(),
                    name: layer_name.to_string(),
                    source: source.to_string(),
                    geom_type: geom_type.to_string(),
                    crs: "EPSG:4326".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer) {
                    Ok(()) => {
                        let rows: Vec<store::StoredFeature> = feats
                            .iter()
                            .map(|f| store::StoredFeature {
                                layer_id: layer_id.to_string(),
                                name: f.name.clone(),
                                kind: f.kind.clone(),
                                lng: f.lng,
                                lat: f.lat,
                                geom_json: f.geom_json.clone(),
                            })
                            .collect();
                        match store::replace_features(&conn, layer_id, &rows) {
                            Ok(n) => format!("imported {n} into {layer_id}"),
                            Err(e) => format!("features failed: {e}"),
                        }
                    }
                    Err(e) => format!("layer failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(layer_id.to_string());
        self.refresh();
        self.fit_only = Some(layer_id.to_string());
        self.fit_requested = true;
        self.push_log(outcome);
    }

    /// Vector > Add Vector Layer (GeoJSON via file picker).
    fn import_geojson_file(&mut self, path: &std::path::Path) {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "import".to_string());
        let layer_id = slug(&stem);
        match std::fs::read_to_string(path) {
            Ok(text) => match import::parse_geojson(&text) {
                Ok(feats) => {
                    let geom = feats
                        .iter()
                        .find_map(|f| f.geom_json.as_ref())
                        .map(|g| {
                            if g.contains("\"Polygon\"") {
                                "polygon"
                            } else {
                                "line"
                            }
                        })
                        .unwrap_or("point");
                    self.persist_batch(&layer_id, &stem, "import", geom, &feats);
                }
                Err(e) => self.push_log(format!("parse failed: {e}")),
            },
            Err(e) => self.push_log(format!("read failed: {e}")),
        }
    }

    /// Vector > Add Delimited Text Layer (CSV via file picker).
    fn import_csv_file(&mut self, path: &std::path::Path) {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "import".to_string());
        let layer_id = slug(&stem);
        match std::fs::read_to_string(path) {
            Ok(text) => match import::parse_csv(&text, None, None) {
                Ok(feats) => self.persist_batch(&layer_id, &stem, "import", "point", &feats),
                Err(e) => self.push_log(format!("parse failed: {e}")),
            },
            Err(e) => self.push_log(format!("read failed: {e}")),
        }
    }

    /// Vector > Export Layer as GeoJSON (selected layer).
    fn export_geojson_file(&mut self, path: &std::path::Path) {
        let Some(id) = self.selected.clone() else {
            self.push_log("nothing selected".to_string());
            return;
        };
        let outcome = match store::open_default() {
            Ok(conn) => match store::list_features(&conn, Some(id.as_str())) {
                Ok(feats) => {
                    let pairs: Vec<(String, String, f64, f64, Option<String>)> = feats
                        .iter()
                        .map(|f| {
                            (
                                f.name.clone(),
                                f.kind.clone(),
                                f.lng,
                                f.lat,
                                f.geom_json.clone(),
                            )
                        })
                        .collect();
                    let refs: Vec<(&str, &str, f64, f64, Option<&str>)> = pairs
                        .iter()
                        .map(|(n, k, lng, lat, g)| {
                            (n.as_str(), k.as_str(), *lng, *lat, g.as_deref())
                        })
                        .collect();
                    match import::export_geojson(&refs) {
                        Ok(text) => match std::fs::write(path, text) {
                            Ok(()) => format!("exported {} features", pairs.len()),
                            Err(e) => format!("write failed: {e}"),
                        },
                        Err(e) => format!("export failed: {e}"),
                    }
                }
                Err(e) => format!("list failed: {e}"),
            },
            Err(e) => format!("db open failed: {e}"),
        };
        self.push_log(outcome);
    }

    /// Raster > Add XYZ Tile Layer (validated template).
    fn add_xyz_layer(&mut self, name: &str, url: &str) {
        if name.trim().is_empty() {
            self.push_log("name must not be empty".to_string());
            return;
        }
        if !tiles::is_usable_template(url) {
            self.push_log("URL must be http(s) with {z}/{x}/{y}".to_string());
            return;
        }
        let id = slug(name);
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: id.clone(),
                    name: name.trim().to_string(),
                    source: format!("xyz:{url}"),
                    geom_type: "raster".to_string(),
                    crs: "EPSG:3857".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer) {
                    Ok(()) => format!("xyz layer {id} added"),
                    Err(e) => format!("add failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.refresh();
        self.push_log(outcome);
    }

    /// Database > New Scratch Layer (empty, QGIS equivalent).
    fn add_scratch_layer(&mut self, name: &str, geom: &str) {
        if name.trim().is_empty() {
            self.push_log("name must not be empty".to_string());
            return;
        }
        if !["point", "line", "polygon"].contains(&geom) {
            self.push_log("geometry must be point/line/polygon".to_string());
            return;
        }
        let id = slug(name);
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer::new(
                    id.clone(),
                    name.trim().to_string(),
                    Some("scratch".to_string()),
                    Some(geom.to_string()),
                    None,
                );
                match store::add_layer(&conn, &layer) {
                    Ok(()) => format!("scratch layer {id} ({geom}) ready"),
                    Err(e) => format!("add failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.selected = Some(id);
        self.refresh();
        self.push_log(outcome);
    }

    /// Web > [preset]: swap the basemap to a known XYZ endpoint.
    fn use_xyz_preset(&mut self, id: &str, url: &str) {
        let outcome = match store::open_default() {
            Ok(conn) => {
                let layer = store::StoredLayer {
                    id: id.to_string(),
                    name: id.to_string(),
                    source: format!("xyz:{url}"),
                    geom_type: "raster".to_string(),
                    crs: "EPSG:3857".to_string(),
                    visible: true,
                    color: String::new(),
                };
                match store::add_layer(&conn, &layer) {
                    Ok(()) => {
                        if let Ok(layers) = store::list_layers(&conn) {
                            for l in layers {
                                if l.geom_type == "raster" && l.id != id {
                                    let _ = store::set_visible(&conn, l.id.as_str(), false);
                                }
                            }
                        }
                        format!("basemap {id}")
                    }
                    Err(e) => format!("add failed: {e}"),
                }
            }
            Err(e) => format!("db open failed: {e}"),
        };
        self.refresh();
        self.push_log(outcome);
    }

    /// Processing: OSM download in a worker thread (UI never freezes).
    /// Result lands via `pending`, ingested on the next frame.
    fn start_osm_download(&mut self, key: &str, value: &str, use_view: bool) {
        if key.trim().is_empty() || value.trim().is_empty() {
            self.push_log("key and value must not be empty".to_string());
            return;
        }
        let bbox = if use_view { self.view_bbox() } else { None };
        let (key, value) = (key.trim().to_string(), value.trim().to_string());
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        self.push_log(format!("downloading {key}={value} ..."));
        std::thread::spawn(move || {
            let out: Result<IngestBatch, String> = (|| {
                let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime: {e}"))?;
                let ql = giscraft_core::overpass::build_tag_query(&key, &value, bbox)
                    .map_err(|e| e.to_string())?;
                let hits = rt
                    .block_on(giscraft_core::overpass::execute_ql(&ql))
                    .map_err(|e| e.to_string())?;
                let slug = slug(&format!("{key}-{value}"));
                let feats = hits
                    .into_iter()
                    .map(|h| store::StoredFeature {
                        layer_id: slug.clone(),
                        name: h.name,
                        kind: h.kind,
                        lng: h.lng,
                        lat: h.lat,
                        geom_json: h.geom.map(|g| g.to_string()),
                    })
                    .collect();
                Ok(IngestBatch {
                    layer_id: slug.clone(),
                    layer_name: format!("{key}={value}"),
                    features: feats,
                })
            })();
            let _ = tx.send(out);
        });
    }

    /// Processing: geocode places to a point layer (worker thread).
    fn start_geocode(&mut self, query: &str, limit: u8) {
        if query.trim().is_empty() {
            self.push_log("query must not be empty".to_string());
            return;
        }
        if limit == 0 || limit > 50 {
            self.push_log("limit must be 1..50".to_string());
            return;
        }
        let (query, limit) = (query.trim().to_string(), limit);
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        self.push_log(format!("geocoding '{query}' ..."));
        std::thread::spawn(move || {
            let out: Result<IngestBatch, String> = (|| {
                let rt = tokio::runtime::Runtime::new().map_err(|e| format!("runtime: {e}"))?;
                let hits = rt
                    .block_on(giscraft_core::geocode::photon_search(&query, limit))
                    .map_err(|e| e.to_string())?;
                let slug = format!(
                    "geocode-{}",
                    query
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                );
                let feats = hits
                    .into_iter()
                    .map(|h| store::StoredFeature {
                        layer_id: slug.clone(),
                        name: h.name,
                        kind: "node".to_string(),
                        lng: h.lng,
                        lat: h.lat,
                        geom_json: None,
                    })
                    .collect();
                Ok(IngestBatch {
                    layer_id: slug,
                    layer_name: query.clone(),
                    features: feats,
                })
            })();
            let _ = tx.send(out);
        });
    }

    /// Processing: buffer the selected layer's points (worker thread).
    /// Polygons out, same centroid in. Non-point features pass through.
    fn run_buffer(&mut self, radius_m: f64) {
        if !radius_m.is_finite() || radius_m <= 0.0 || radius_m > 1_000_000.0 {
            self.push_log("radius must be 0..1000000 m".to_string());
            return;
        }
        let Some(id) = self.selected.clone() else {
            self.push_log("nothing selected".to_string());
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        self.pending = Some(rx);
        self.push_log(format!("buffering {id} by {radius_m} m ..."));
        std::thread::spawn(move || {
            let out: Result<IngestBatch, String> = (|| {
                let conn = store::open_default().map_err(|e| e.to_string())?;
                let feats =
                    store::list_features(&conn, Some(id.as_str())).map_err(|e| e.to_string())?;
                let raw: Vec<giscraft_core::import::RawFeature> = feats
                    .iter()
                    .map(|f| giscraft_core::import::RawFeature {
                        name: f.name.clone(),
                        kind: f.kind.clone(),
                        lng: f.lng,
                        lat: f.lat,
                        geom_json: f.geom_json.clone(),
                    })
                    .collect();
                let buffered = giscraft_core::processing::buffer_features(&raw, radius_m, 32)
                    .map_err(|e| e.to_string())?;
                let features: Vec<store::StoredFeature> = buffered
                    .into_iter()
                    .map(|f| store::StoredFeature {
                        layer_id: String::new(),
                        name: f.name,
                        kind: "way".to_string(),
                        lng: f.lng,
                        lat: f.lat,
                        geom_json: f.geom_json,
                    })
                    .collect();
                Ok(IngestBatch {
                    layer_id: format!("{id}-buffer"),
                    layer_name: format!("{id} buffered {radius_m}m"),
                    features,
                })
            })();
            let _ = tx.send(out);
        });
    }

    /// Current view bbox in lng/lat (for "use map canvas extent").
    fn view_bbox(&self) -> Option<giscraft_core::geo::Bbox> {
        let rect = self.last_rect?;
        let (wlng, nlat) = unproject(rect.min, self.center_lng, self.center_lat, self.zoom, &rect);
        let (elng, slat) = unproject(rect.max, self.center_lng, self.center_lat, self.zoom, &rect);
        giscraft_core::geo::Bbox::new(wlng, slat, elng, nlat).ok()
    }

    /// Drain finished background downloads into the project DB.
    fn poll_pending(&mut self) {
        let done = if let Some(rx) = self.pending.as_ref() {
            rx.try_recv().ok()
        } else {
            None
        };
        if let Some(result) = done {
            self.pending = None;
            match result {
                Ok(batch) => {
                    let n = batch.features.len();
                    let outcome = match store::open_default() {
                        Ok(conn) => {
                            let layer = store::StoredLayer {
                                id: batch.layer_id.clone(),
                                name: batch.layer_name.clone(),
                                source: "processing".to_string(),
                                geom_type: "point".to_string(),
                                crs: "EPSG:4326".to_string(),
                                visible: true,
                                color: String::new(),
                            };
                            match store::add_layer(&conn, &layer).and_then(|()| {
                                store::replace_features(
                                    &conn,
                                    batch.layer_id.as_str(),
                                    &batch.features,
                                )
                                .map(|_| ())
                            }) {
                                Ok(()) => format!("downloaded {n} into {}", batch.layer_id),
                                Err(e) => format!("ingest failed: {e}"),
                            }
                        }
                        Err(e) => format!("db open failed: {e}"),
                    };
                    self.selected = Some(batch.layer_id.clone());
                    self.refresh();
                    self.fit_only = Some(batch.layer_id);
                    self.fit_requested = true;
                    self.push_log(outcome);
                }
                Err(e) => self.push_log(format!("download failed: {e}")),
            }
        }
    }

    /// Render the open modal dialog, if any. Dialogs close on submit or
    /// cancel; the outcome always lands in the log. The dialog state is
    /// taken out of `self` first so arms can freely call `&mut self`.
    fn show_dialog(&mut self, ui: &mut egui::Ui) {
        if matches!(self.dialog, Dialog::None) {
            return;
        }
        let mut dlg = std::mem::replace(&mut self.dialog, Dialog::None);
        let mut keep = true;
        match &mut dlg {
            Dialog::None => {}
            Dialog::XyzTile { name, url } => {
                let mut submit: Option<(String, String)> = None;
                let mut cancel = false;
                egui::Window::new("Add XYZ Tile Layer").show(ui.ctx(), |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(name);
                    ui.label("URL template ({z}/{x}/{y}):");
                    ui.text_edit_singleline(url);
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            submit = Some((name.clone(), url.clone()));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some((n, u)) = submit {
                    self.add_xyz_layer(&n, &u);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Scratch { name, geom } => {
                let mut submit: Option<(String, String)> = None;
                let mut cancel = false;
                egui::Window::new("New Scratch Layer").show(ui.ctx(), |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(name);
                    ui.label("Geometry:");
                    for (i, kind) in GEOM_KINDS.iter().enumerate() {
                        if ui.selectable_label(*geom == i, *kind).clicked() {
                            *geom = i;
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let g = GEOM_KINDS.get(*geom).unwrap_or(&"point").to_string();
                            submit = Some((name.clone(), g));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some((n, g)) = submit {
                    self.add_scratch_layer(&n, &g);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::OsmDownload {
                key,
                value,
                use_view,
            } => {
                let mut submit: Option<(String, String, bool)> = None;
                let mut cancel = false;
                egui::Window::new("Download OSM features").show(ui.ctx(), |ui| {
                    ui.label("Tag key:");
                    ui.text_edit_singleline(key);
                    ui.label("Tag value:");
                    ui.text_edit_singleline(value);
                    ui.label("Common POIs (tap to fill):");
                    ui.horizontal_wrapped(|ui| {
                        for (label, k, v) in [
                            ("Cafes", "amenity", "cafe"),
                            ("Pharmacies", "amenity", "pharmacy"),
                            ("Bike shops", "shop", "bicycle"),
                            ("Restaurants", "amenity", "restaurant"),
                            ("Bars", "amenity", "bar"),
                            ("ATMs", "amenity", "atm"),
                            ("Locksmiths", "craft", "locksmith"),
                            ("Bakeries", "shop", "bakery"),
                            ("Supermarkets", "shop", "supermarket"),
                        ] {
                            if ui.button(label).clicked() {
                                *key = k.to_string();
                                *value = v.to_string();
                            }
                        }
                    });
                    ui.checkbox(use_view, "Restrict to map canvas extent");
                    ui.horizontal(|ui| {
                        if ui.button("Download").clicked() {
                            submit = Some((key.clone(), value.clone(), *use_view));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some((k, v, uv)) = submit {
                    self.start_osm_download(&k, &v, uv);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Geocode { query, limit } => {
                let mut submit: Option<(String, u8)> = None;
                let mut cancel = false;
                egui::Window::new("Geocode places to layer").show(ui.ctx(), |ui| {
                    ui.label("Place query:");
                    ui.text_edit_singleline(query);
                    ui.label("Limit (1-50):");
                    ui.text_edit_singleline(limit);
                    ui.horizontal(|ui| {
                        if ui.button("Geocode").clicked() {
                            let lim = limit.trim().parse::<u8>().unwrap_or(0);
                            submit = Some((query.clone(), lim));
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some((q, lim)) = submit {
                    if lim == 0 || lim > 50 {
                        self.push_log("limit must be 1..50".to_string());
                    } else {
                        self.start_geocode(&q, lim);
                        keep = false;
                    }
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Wkt { text, layer_id } => {
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("Show WKT as layer").show(ui.ctx(), |ui| {
                    ui.label("WKT (POINT / LINESTRING / POLYGON + MULTI, 2D):");
                    ui.text_edit_multiline(text);
                    ui.label("Layer id:");
                    ui.text_edit_singleline(layer_id);
                    ui.horizontal(|ui| {
                        if ui.button("Show").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if submit {
                    let id = if layer_id.trim().is_empty() {
                        "wkt".to_string()
                    } else {
                        layer_id.trim().to_string()
                    };
                    let body = text.clone();
                    self.submit_wkt(&body, &id);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::ZoomTo { text } => {
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("Zoom to coordinates").show(ui.ctx(), |ui| {
                    ui.label("lat, lng - e.g. 48.2082, 16.3738 (N/S/E/W suffixes ok):");
                    ui.text_edit_singleline(text);
                    ui.horizontal(|ui| {
                        if ui.button("Go").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if submit {
                    let body = text.clone();
                    self.zoom_to_coords(&body);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Grid {
                south,
                west,
                north,
                east,
                cell_size_m,
                hex,
                layer_id,
            } => {
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("Create grid").show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("South:");
                        ui.text_edit_singleline(south);
                        ui.label("West:");
                        ui.text_edit_singleline(west);
                    });
                    ui.horizontal(|ui| {
                        ui.label("North:");
                        ui.text_edit_singleline(north);
                        ui.label("East:");
                        ui.text_edit_singleline(east);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Cell size (m):");
                        ui.text_edit_singleline(cell_size_m);
                        ui.checkbox(hex, "Hex (unchecked = rectangles)");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Layer id:");
                        ui.text_edit_singleline(layer_id);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Build").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if submit {
                    let args = (
                        south.clone(),
                        west.clone(),
                        north.clone(),
                        east.clone(),
                        cell_size_m.clone(),
                        *hex,
                        if layer_id.trim().is_empty() {
                            "grid".to_string()
                        } else {
                            layer_id.trim().to_string()
                        },
                    );
                    self.submit_grid(args);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Nearest {
                source,
                target,
                output,
            } => {
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("Nearest neighbor join").show(ui.ctx(), |ui| {
                    ui.label("Source layer (hubs start here):");
                    ui.text_edit_singleline(source);
                    ui.label("Target layer (nearest match):");
                    ui.text_edit_singleline(target);
                    ui.label("Output layer (empty = <source>-nearest):");
                    ui.text_edit_singleline(output);
                    ui.horizontal(|ui| {
                        if ui.button("Join").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if submit {
                    let args = (source.clone(), target.clone(), output.clone());
                    self.submit_nearest(args);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Geodesic {
                shape,
                lng,
                lat,
                radius_m,
                major_m,
                minor_m,
                bearing,
                lng2,
                lat2,
                layer_id,
            } => {
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("Geodesic shape").show(ui.ctx(), |ui| {
                    ui.label("Shape: circle, ellipse or line:");
                    ui.text_edit_singleline(shape);
                    ui.horizontal(|ui| {
                        ui.label("Center lng:");
                        ui.text_edit_singleline(lng);
                        ui.label("Center lat:");
                        ui.text_edit_singleline(lat);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Circle radius (m):");
                        ui.text_edit_singleline(radius_m);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Ellipse major/minor/bearing:");
                        ui.text_edit_singleline(major_m);
                        ui.text_edit_singleline(minor_m);
                        ui.text_edit_singleline(bearing);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Line end lng/lat:");
                        ui.text_edit_singleline(lng2);
                        ui.text_edit_singleline(lat2);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Layer id:");
                        ui.text_edit_singleline(layer_id);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Build").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if submit {
                    let args = (
                        shape.clone(),
                        lng.clone(),
                        lat.clone(),
                        radius_m.clone(),
                        major_m.clone(),
                        minor_m.clone(),
                        bearing.clone(),
                        lng2.clone(),
                        lat2.clone(),
                        if layer_id.trim().is_empty() {
                            "geodesic".to_string()
                        } else {
                            layer_id.trim().to_string()
                        },
                    );
                    self.submit_geodesic(args);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Doppler { capture, layer_id } => {
                let mut submit = false;
                let mut cancel = false;
                let mut picked: Option<String> = None;
                egui::Window::new("Doppler locate").show(ui.ctx(), |ui| {
                    ui.label("Capture JSON (tracks + curves):");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(capture);
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("JSON", &["json"])
                                .pick_file()
                            {
                                picked = Some(path.to_string_lossy().to_string());
                            }
                        }
                    });
                    ui.label("Output layer (bbox = current canvas):");
                    ui.text_edit_singleline(layer_id);
                    ui.horizontal(|ui| {
                        if ui.button("Locate").clicked() {
                            submit = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some(p) = picked {
                    *capture = p;
                }
                if submit {
                    let id = if layer_id.trim().is_empty() {
                        "doppler-hits".to_string()
                    } else {
                        layer_id.trim().to_string()
                    };
                    let path = capture.clone();
                    self.submit_doppler(&path, &id);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
            Dialog::Buffer { radius_m } => {
                let mut cancel = false;
                let mut submit: Option<f64> = None;
                egui::Window::new("Buffer points").show(ui.ctx(), |ui| {
                    ui.label("Radius in meters (selected layer, points only):");
                    ui.text_edit_singleline(radius_m);
                    ui.horizontal(|ui| {
                        if ui.button("Buffer").clicked() {
                            if let Ok(r) = radius_m.trim().parse::<f64>() {
                                submit = Some(r);
                            } else {
                                self.push_log("radius must be a number".to_string());
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some(r) = submit {
                    self.run_buffer(r);
                    keep = false;
                } else if cancel {
                    keep = false;
                }
            }
        }
        if keep {
            self.dialog = dlg;
        }
    }

    fn push_log(&mut self, line: String) {
        self.log.push(line);
        while self.log.len() > 50 {
            self.log.remove(0);
        }
    }
}

impl Default for GisCraftApp {
    fn default() -> Self {
        Self::open_with_seed(None)
    }
}

impl GisCraftApp {
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
            "layer.color" => {
                let id = str_param("id");
                let color = str_param("color");
                if id.trim().is_empty() {
                    return serde_json::json!({"error": "invalid params: id required"});
                }
                match tiles::parse_layer_color(&color) {
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                    Ok(_) => match store::open_default()
                        .and_then(|c| store::set_color(&c, id.as_str(), color.trim()))
                    {
                        Ok(updated) => {
                            self.refresh();
                            serde_json::json!({"id": id, "color": color.trim(), "updated": updated})
                        }
                        Err(e) => serde_json::json!({"error": e.to_string()}),
                    },
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
                                    source: l.source,
                                    geom_type: l.geom_type,
                                    visible: l.visible,
                                    color: l.color,
                                    features: n,
                                }
                            })
                            .collect();
                        let tmpl = resolve_tile_template(&rows);
                        match render_preview(&feats, &rows, w, h, with_tiles, &tmpl) {
                            Ok(bytes) => {
                                let mut path = std::env::temp_dir();
                                path.push(format!("giscraft-control-{}.bmp", std::process::id()));
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
            "processing.buffer" => {
                let layer_id = str_param("layer_id");
                let distance_m = params
                    .get("distance")
                    .or_else(|| params.get("distance_m"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(200.0);
                if layer_id.trim().is_empty() {
                    return serde_json::json!({"error": "invalid params: layer_id required"});
                }
                if distance_m <= 0.0 || !distance_m.is_finite() || distance_m > 1_000_000.0 {
                    return serde_json::json!({"error": "invalid params: distance must be 0..1000000 m"});
                }
                let segments = params
                    .get("segments")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| usize::try_from(n).ok())
                    .unwrap_or(24);
                let output_layer = opt_str("output_layer");
                let conn = match store::open_default() {
                    Ok(c) => c,
                    Err(e) => return serde_json::json!({"error": e.to_string()}),
                };
                let feats = match store::list_features(&conn, Some(layer_id.as_str())) {
                    Ok(f) => f,
                    Err(e) => return serde_json::json!({"error": e.to_string()}),
                };
                if feats.is_empty() {
                    return serde_json::json!({"error": format!("layer not found or empty: {layer_id}")});
                }
                let raw: Vec<giscraft_core::import::RawFeature> = feats
                    .into_iter()
                    .map(|f| giscraft_core::import::RawFeature {
                        name: f.name,
                        kind: f.kind,
                        lng: f.lng,
                        lat: f.lat,
                        geom_json: f.geom_json,
                    })
                    .collect();
                let buffered =
                    match giscraft_core::processing::buffer_features(&raw, distance_m, segments) {
                        Ok(b) => b,
                        Err(e) => return serde_json::json!({"error": e.to_string()}),
                    };
                let out_id = output_layer.unwrap_or_else(|| format!("{layer_id}-buffer"));
                let out_name = format!("{layer_id} buffer ({distance_m}m)");
                let new_layer = store::StoredLayer::new(
                    out_id.clone(),
                    out_name,
                    Some("processing:buffer".to_string()),
                    Some("polygon".to_string()),
                    None,
                );
                if let Err(e) = store::add_layer(&conn, &new_layer) {
                    return serde_json::json!({"error": e.to_string()});
                }
                let stored_feats: Vec<store::StoredFeature> = buffered
                    .into_iter()
                    .map(|f| store::StoredFeature {
                        layer_id: out_id.clone(),
                        name: f.name,
                        kind: f.kind,
                        lng: f.lng,
                        lat: f.lat,
                        geom_json: f.geom_json,
                    })
                    .collect();
                match store::replace_features(&conn, &out_id, &stored_feats) {
                    Ok(n) => {
                        self.refresh();
                        serde_json::json!({
                            "input_layer": layer_id,
                            "output_layer": out_id,
                            "distance_m": distance_m,
                            "buffered": n,
                        })
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            _ => serde_json::json!({"error": format!("unknown method: {method}")}),
        }
    }
}

impl eframe::App for GisCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_control();
        self.poll_pending();
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
                let loc = self.locale;
                use giscraft_core::i18n::{t, I18nKey};

                ui.menu_button(t(I18nKey::MenuProject, loc), |ui| {
                    let label = if self.confirm_new {
                        "New (click again to confirm wipe)"
                    } else {
                        t(I18nKey::ProjectNew, loc)
                    };
                    if ui.button(label).clicked() {
                        if self.confirm_new {
                            self.new_project();
                        } else {
                            self.confirm_new = true;
                            self.push_log("New armed: click again to wipe".to_string());
                        }
                    }
                    if ui.button(t(I18nKey::ProjectRefresh, loc)).clicked() {
                        self.confirm_new = false;
                        self.refresh();
                        self.push_log("refreshed".to_string());
                    }
                    if ui.button(t(I18nKey::ProjectQuit, loc)).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button(t(I18nKey::MenuLayer, loc), |ui| {
                    if ui.button(t(I18nKey::LayerAddVector, loc)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GeoJSON", &["geojson", "json"])
                            .pick_file()
                        {
                            self.import_geojson_file(&path);
                        } else {
                            self.push_log("import cancelled".to_string());
                        }
                    }
                    if ui.button(t(I18nKey::LayerAddCsv, loc)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV", &["csv", "txt"])
                            .pick_file()
                        {
                            self.import_csv_file(&path);
                        } else {
                            self.push_log("import cancelled".to_string());
                        }
                    }
                    if ui.button(t(I18nKey::LayerRemove, loc)).clicked() {
                        self.remove_selected();
                    }
                    if ui.button(t(I18nKey::LayerZoomTo, loc)).clicked() {
                        self.zoom_to_selected();
                    }
                    if ui.button(t(I18nKey::LayerShowAll, loc)).clicked() {
                        self.set_all_visible(true);
                    }
                    if ui.button(t(I18nKey::LayerHideAll, loc)).clicked() {
                        self.set_all_visible(false);
                    }
                });
                ui.menu_button(t(I18nKey::MenuView, loc), |ui| {
                    if ui.button(t(I18nKey::ViewZoomIn, loc)).clicked() {
                        self.zoom = (self.zoom + 1.0).clamp(2.0, 19.0);
                        self.status = "zoom".to_string();
                    }
                    if ui.button(t(I18nKey::ViewZoomOut, loc)).clicked() {
                        self.zoom = (self.zoom - 1.0).clamp(2.0, 19.0);
                        self.status = "zoom".to_string();
                    }
                    if ui.button(t(I18nKey::ViewFitData, loc)).clicked() {
                        self.fit_only = None;
                        self.fit_requested = true;
                    }
                    if ui.button(t(I18nKey::ViewResetVienna, loc)).clicked() {
                        self.center_lng = 16.3738;
                        self.center_lat = 48.2082;
                        self.zoom = 15.0;
                        self.status = "reset".to_string();
                    }
                });
                ui.menu_button(t(I18nKey::MenuSettings, loc), |ui| {
                    if ui.button("Configuration...").clicked() {
                        self.show_settings = true;
                    }
                });
                ui.menu_button(t(I18nKey::MenuHelp, loc), |ui| {
                    if ui.button("GISCraft Handbook & GIS Guide...").clicked() {
                        self.show_help = true;
                    }
                    ui.separator();
                    if ui.button("About giscraft").clicked() {
                        self.show_about = true;
                    }
                });
                ui.menu_button(t(I18nKey::MenuVector, loc), |ui| {
                    if ui.button("Add Vector Layer...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GeoJSON", &["geojson", "json"])
                            .pick_file()
                        {
                            self.import_geojson_file(&path);
                        } else {
                            self.push_log("import cancelled".to_string());
                        }
                    }
                    if ui.button("Add Delimited Text Layer...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV", &["csv", "txt"])
                            .pick_file()
                        {
                            self.import_csv_file(&path);
                        } else {
                            self.push_log("import cancelled".to_string());
                        }
                    }
                    if ui.button("Export Layer as GeoJSON...").clicked() {
                        if self.selected.is_none() {
                            self.push_log("nothing selected".to_string());
                        } else if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GeoJSON", &["geojson"])
                            .set_file_name("layer.geojson")
                            .save_file()
                        {
                            self.export_geojson_file(&path);
                        } else {
                            self.push_log("export cancelled".to_string());
                        }
                    }
                });
                ui.menu_button(t(I18nKey::MenuRaster, loc), |ui| {
                    if ui.button("Add XYZ Tile Layer...").clicked() {
                        self.dialog = Dialog::XyzTile {
                            name: String::new(),
                            url: tiles::tile_url_template(),
                        };
                    }
                    if ui.button("Remove selected raster").clicked() {
                        self.remove_selected();
                    }
                });
                ui.menu_button(t(I18nKey::MenuDatabase, loc), |ui| {
                    if ui.button("New Scratch Layer...").clicked() {
                        self.dialog = Dialog::Scratch {
                            name: String::new(),
                            geom: 0,
                        };
                    }
                });
                ui.menu_button(t(I18nKey::MenuWeb, loc), |ui| {
                    ui.label("XYZ basemap presets (one click, replaces current):");
                    for (id, url) in XYZ_PRESETS {
                        if ui.button(id).clicked() {
                            self.use_xyz_preset(id, url);
                        }
                    }
                });
                ui.menu_button(t(I18nKey::MenuProcessing, loc), |ui| {
                    if ui.button("Download OSM features...").clicked() {
                        self.dialog = Dialog::OsmDownload {
                            key: "amenity".to_string(),
                            value: "pharmacy".to_string(),
                            use_view: true,
                        };
                    }
                    if ui.button("Geocode places to layer...").clicked() {
                        self.dialog = Dialog::Geocode {
                            query: String::new(),
                            limit: "5".to_string(),
                        };
                    }
                });
                ui.menu_button(t(I18nKey::MenuEdit, loc), |ui| {
                    ui.label("Edit commands land with later phases (typed, not stubbed)");
                });
                ui.menu_button(t(I18nKey::MenuPlugins, loc), |ui| {
                    if ui.button("Show WKT...").clicked() {
                        self.dialog = Dialog::Wkt {
                            text: String::new(),
                            layer_id: "wkt".to_string(),
                        };
                    }
                    if ui.button("Zoom to coordinates...").clicked() {
                        self.dialog = Dialog::ZoomTo {
                            text: String::new(),
                        };
                    }
                });

                // Top-right Fleet Standard i18n Dropdown
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cur_loc = self.locale;
                    let label = format!("{} {}", cur_loc.flag(), cur_loc.code().to_uppercase());
                    egui::ComboBox::from_id_salt("topbar_locale_selector")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for loc_item in giscraft_core::i18n::AppLocale::all() {
                                let text =
                                    format!("{} {}", loc_item.flag(), loc_item.display_name());
                                let _ = ui.selectable_value(&mut self.locale, *loc_item, text);
                            }
                        });
                });
            });
        });
        if self.show_help {
            egui::Window::new("📖 GISCraft Comprehensive Handbook & Spatial Guide")
                .min_width(680.0)
                .min_height(480.0)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("1. What is GIS? (Geographic Information Systems)");
                        ui.label("A Geographic Information System (GIS) captures, stores, analyzes, and visualizes spatial data on Earth.");
                        ui.label("• Vector Data: Points (POIs), Linestrings (roads/trails), Polygons (districts, borders, water).");
                        ui.label("• Raster Data: Map tiles, elevation DEMs, satellite imagery (Web Mercator EPSG:3857).");
                        ui.label("• CRS (Coordinate Reference System): EPSG:4326 (WGS84 GPS degrees) reprojected to EPSG:3857.");

                        ui.add_space(8.0);
                        ui.heading("2. What is QGIS & Who Uses It?");
                        ui.label("QGIS is the global standard open-source GIS desktop suite (the sovereign counterpart to Esri ArcGIS).");
                        ui.label("Used worldwide by urban planners, civil engineers, foresters, environmental scientists,");
                        ui.label("disaster response teams (Humanitarian OSM), intelligence analysts, and outdoor explorers.");

                        ui.add_space(8.0);
                        ui.heading("3. What is GISCraft?");
                        ui.label("A sovereign, clean-room 1:1 QGIS clone shell written in pure Rust (egui 0.37).");
                        ui.label("• Preserves QGIS muscle memory, menu layouts, shortcuts (Ctrl+Shift+F, F5), and dock interactions.");
                        ui.label("• Database-as-Project: Entire project state lives in one self-contained SQLite file (giscraft.db).");
                        ui.label("• Agentic AI & MCP First: 100% of spatial functions can be operated headlessly by AI via MCP.");

                        ui.add_space(8.0);
                        ui.heading("4. Key Features Existing & Operational");
                        ui.label("• Keyless Photon Geocoding & Offline Vienna Index (Stephansplatz, Karlsplatz, etc.).");
                        ui.label("• Live OpenStreetMap Overpass Harvesting (query any tag, e.g. amenity=pharmacy, and ingest).");
                        ui.label("• Geodesic Spatial Toolbox: Great-circle buffers, circles, ellipses, nearest-neighbor, grids.");
                        ui.label("• SIGINT / Doppler Emitter Localization: Triangulates radio transmitters from satellite flyovers.");
                        ui.label("• Ear-Clipping Vector Polygon Triangulation & Tile Basemap Caching.");

                        ui.add_space(8.0);
                        ui.heading("5. Planned Features & Fleet Synergy");
                        ui.label("• Chrono-Cartography Bridge with TravelCraft: Overlay Edo-period roads over modern Japan.");
                        ui.label("• Native Digital Elevation Model (DEM) Hillshading & Contour lines.");
                        ui.label("• Direct GPX / KML outdoor track ingestion.");

                        ui.add_space(10.0);
                        if ui.button("Close Handbook").clicked() {
                            self.show_help = false;
                        }
                    });
                });
        }
        if self.show_about {
            egui::Window::new("About giscraft").show(ui.ctx(), |ui| {
                ui.label(format!("giscraft-mcp {}", env!("CARGO_PKG_VERSION")));
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
                    giscraft_core::tiles::tile_url_template()
                ));
                ui.monospace(format!(
                    "photon     : {}",
                    giscraft_core::geocode::photon_base()
                ));
                ui.monospace(format!(
                    "overpass   : {}",
                    giscraft_core::overpass::overpass_url()
                ));
                ui.label("Override via environment before launch:");
                ui.monospace("GISCRAFT_DB / GISCRAFT_TILES_URL / GISCRAFT_PHOTON_URL / GISCRAFT_OVERPASS_URL");
                ui.label("Restart the app to apply changes.");
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
        }
        self.show_dialog(ui);

        // Bottom status + log (QGIS status bar + Log Messages parity)
        egui::Panel::bottom("statusbar").show(ui, |ui| {
            let db_name = std::path::Path::new(&self.db_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.db_path.clone());
            ui.horizontal(|ui| {
                // Web Mercator scale at 96 dpi: meters/px * px/m.
                let mpp = tiles::meters_per_pixel(self.center_lat, self.zoom);
                let (bar_m, _bar_px) = tiles::nice_scale_bar(mpp, 140.0);
                let bar_label = if bar_m >= 1000.0 {
                    format!("{:.0} km", bar_m / 1000.0)
                } else {
                    format!("{:.0} m", bar_m)
                };
                ui.label(format!(
                    "{} | EPSG:3857 | z{:.1} 1:{} | bar {} | {}",
                    db_name,
                    self.zoom,
                    (mpp * 3779.527).round() as u64,
                    bar_label,
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
                    ui.colored_label(self.layer_draw_color(&id), "●");
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
            if ui.button("Add OSM data...").clicked() {
                self.dialog = Dialog::OsmDownload {
                    key: String::new(),
                    value: String::new(),
                    use_view: true,
                };
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
            if let Some(id) = self.selected.clone() {
                let stored = self
                    .layers
                    .iter()
                    .find(|l| l.id == id)
                    .map(|l| l.color.clone())
                    .unwrap_or_default();
                let (r, g, b) = tiles::parse_layer_color(&stored)
                    .unwrap_or(None)
                    .unwrap_or_else(|| tiles::layer_color(&id));
                let mut rgb = [r, g, b];
                ui.label(format!("Single symbol: {id}"));
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                    match store::open_default().and_then(|c| store::set_color(&c, &id, &hex)) {
                        Ok(true) => {
                            self.refresh();
                            self.push_log(format!("color {id} -> {hex}"));
                        }
                        Ok(false) => self.push_log("color failed: no such layer".to_string()),
                        Err(e) => self.push_log(format!("color failed: {e}")),
                    }
                }
                if !stored.is_empty() && ui.button("Reset to palette").clicked() {
                    match store::open_default().and_then(|c| store::set_color(&c, &id, "")) {
                        Ok(_) => {
                            self.refresh();
                            self.push_log(format!("color {id} reset to palette"));
                        }
                        Err(e) => self.push_log(format!("color failed: {e}")),
                    }
                }
            } else {
                ui.label("Select a layer to style its symbol color.");
            }
            ui.separator();
            ui.heading("Processing Toolbox");
            ui.text_edit_singleline(&mut self.toolbox_query);
            ui.label("Search algorithms. Click to run (dialog opens).");
            let query = self.toolbox_query.trim().to_lowercase();
            let mut open: Option<Dialog> = None;
            for (name, about) in [
                (
                    "Download OSM features",
                    "Overpass tag query into a new layer",
                ),
                (
                    "Geocode places to layer",
                    "Photon search into a point layer",
                ),
                ("Buffer points", "Circle polygons around selected points"),
                ("Create grid", "Rect or hex grid over a bbox"),
                ("Nearest neighbor join", "Hub lines to closest features"),
                ("Geodesic shape", "True-distance circle, ellipse or line"),
                (
                    "Doppler locate",
                    "Match curves to ADS-B tracks (colsto method)",
                ),
            ] {
                if !query.is_empty()
                    && !name.to_lowercase().contains(&query)
                    && !about.to_lowercase().contains(&query)
                {
                    continue;
                }
                if ui.button(name).clicked() {
                    self.push_log(about.to_string());
                    open = Some(match name {
                        "Download OSM features" => Dialog::OsmDownload {
                            key: "amenity".to_string(),
                            value: "pharmacy".to_string(),
                            use_view: true,
                        },
                        "Geocode places to layer" => Dialog::Geocode {
                            query: String::new(),
                            limit: "5".to_string(),
                        },
                        "Create grid" => {
                            let (s, w, n, e) = match self.view_bbox() {
                                Some(b) => (
                                    b.min_lat.to_string(),
                                    b.min_lng.to_string(),
                                    b.max_lat.to_string(),
                                    b.max_lng.to_string(),
                                ),
                                None => (
                                    "48.195".to_string(),
                                    "16.355".to_string(),
                                    "48.225".to_string(),
                                    "16.395".to_string(),
                                ),
                            };
                            Dialog::Grid {
                                south: s,
                                west: w,
                                north: n,
                                east: e,
                                cell_size_m: "500".to_string(),
                                hex: false,
                                layer_id: "grid".to_string(),
                            }
                        }
                        "Nearest neighbor join" => Dialog::Nearest {
                            source: self.selected.clone().unwrap_or_default(),
                            target: String::new(),
                            output: String::new(),
                        },
                        "Geodesic shape" => {
                            let (lng, lat) = match self.view_bbox() {
                                Some(b) => (
                                    ((b.min_lng + b.max_lng) / 2.0).to_string(),
                                    ((b.min_lat + b.max_lat) / 2.0).to_string(),
                                ),
                                None => ("16.3738".to_string(), "48.2082".to_string()),
                            };
                            Dialog::Geodesic {
                                shape: "circle".to_string(),
                                lng,
                                lat,
                                radius_m: "1000".to_string(),
                                major_m: "1000".to_string(),
                                minor_m: "500".to_string(),
                                bearing: "0".to_string(),
                                lng2: String::new(),
                                lat2: String::new(),
                                layer_id: "geodesic".to_string(),
                            }
                        }
                        "Doppler locate" => Dialog::Doppler {
                            capture: String::new(),
                            layer_id: "doppler-hits".to_string(),
                        },
                        _ => Dialog::Buffer {
                            radius_m: "500".to_string(),
                        },
                    });
                }
            }
            if let Some(dlg) = open {
                self.dialog = dlg;
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
            // XYZ underlay from the active raster layer (or the default
            // template). Cache-first sync fetch, max 2 network misses per
            // frame so the UI paints in progressively instead of freezing
            // (async worker is the follow-up; see gaps).
            let template = resolve_tile_template(&self.layers);
            if template != self.tile_template {
                self.tile_template = template.clone();
                self.textures.clear();
                self.push_log("basemap switched (cache cleared)".to_string());
            }
            let tile_z = (self.zoom.floor() as i64).clamp(0, 19) as u8;
            let (wlng, nlat) =
                unproject(rect.min, self.center_lng, self.center_lat, self.zoom, &rect);
            let (elng, slat) =
                unproject(rect.max, self.center_lng, self.center_lat, self.zoom, &rect);
            match tiles::bbox_to_tiles(wlng, slat, elng, nlat, tile_z) {
                Ok(list) => {
                    let mut fetched = 0_usize;
                    for t in list {
                        let key =
                            format!("{}:{}:{}:{}", tiles::template_key(&template), t.z, t.x, t.y);
                        if !self.textures.contains_key(&key) && fetched >= 2 {
                            continue;
                        }
                        let loaded = match self.textures.entry(key.clone()) {
                            std::collections::hash_map::Entry::Occupied(_) => true,
                            std::collections::hash_map::Entry::Vacant(slot) => {
                                fetched += 1;
                                match tiles::fetch_tile_png_as(t, &template)
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
                let color = self.layer_draw_color(feat.layer_id.as_str());
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
                        painter.circle_filled(pos, 5.5, egui::Color32::WHITE);
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
            // Scale bar (bottom-left) + north arrow (top-right): QGIS chrome.
            {
                let mpp = tiles::meters_per_pixel(self.center_lat, self.zoom);
                let (bar_m, bar_px) = tiles::nice_scale_bar(mpp, 140.0);
                if bar_px > 20.0 {
                    let base = rect.left_bottom() + egui::Vec2::new(14.0, -20.0);
                    let end = base + egui::Vec2::new(bar_px as f32, 0.0);
                    painter.line_segment([base, end], egui::Stroke::new(2.0, egui::Color32::WHITE));
                    for x in [base.x, end.x] {
                        painter.line_segment(
                            [
                                egui::Pos2::new(x, base.y - 4.0),
                                egui::Pos2::new(x, base.y + 4.0),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                    }
                    let label = if bar_m >= 1000.0 {
                        format!("{:.0} km", bar_m / 1000.0)
                    } else {
                        format!("{:.0} m", bar_m)
                    };
                    painter.text(
                        end + egui::Vec2::new(6.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                }
                let nc = rect.right_top() + egui::Vec2::new(-30.0, 34.0);
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        nc + egui::Vec2::new(0.0, -10.0),
                        nc + egui::Vec2::new(-6.0, 6.0),
                        nc + egui::Vec2::new(6.0, 6.0),
                    ],
                    egui::Color32::WHITE,
                    egui::Stroke::NONE,
                ));
                painter.text(
                    nc + egui::Vec2::new(0.0, -12.0),
                    egui::Align2::CENTER_BOTTOM,
                    "N",
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
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
                // Grab-and-drag: content follows the cursor, so the new
                // center is the geo that sat one drag-vector up-left.
                let delta = resp.drag_delta();
                let c = rect.center();
                let (nclng, nclat) = unproject(
                    egui::Pos2::new(c.x - delta.x, c.y - delta.y),
                    self.center_lng,
                    self.center_lat,
                    self.zoom,
                    &rect,
                );
                self.center_lng = nclng;
                self.center_lat = nclat;
                self.status = "pan".to_string();
            }
            // Identify on hover + click: topmost visible feature wins.
            // QGIS behavior: tooltip on hover, click selects the layer
            // and pins the details; empty click clears the selection.
            if let Some(pos) = hover {
                if rect.contains(pos) && !resp.dragged() {
                    if let Some(hit) = self.pick_at(pos, &rect) {
                        let detail = format!("{} · {:.5}, {:.5}", hit.kind, hit.lat, hit.lng);
                        let pad = egui::Vec2::new(8.0, 6.0);
                        let w = 260.0;
                        let anchor = egui::Pos2::new(
                            (pos.x + 14.0).min(rect.max.x - w - 4.0),
                            (pos.y + 16.0).min(rect.max.y - 52.0),
                        );
                        let card = egui::Rect::from_min_size(anchor, egui::Vec2::new(w, 44.0));
                        painter.rect_filled(card, 4.0, egui::Color32::from_rgb(0x16, 0x16, 0x1a));
                        painter.rect_stroke(
                            card,
                            4.0,
                            egui::Stroke::new(1.0, egui::Color32::WHITE),
                            egui::StrokeKind::Inside,
                        );
                        painter.text(
                            anchor + egui::Vec2::new(pad.x, pad.y),
                            egui::Align2::LEFT_TOP,
                            hit.name.clone(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
                        painter.text(
                            anchor + egui::Vec2::new(pad.x, pad.y + 16.0),
                            egui::Align2::LEFT_TOP,
                            detail,
                            egui::FontId::proportional(11.0),
                            egui::Color32::WHITE,
                        );
                        if resp.clicked() {
                            self.selected = Some(hit.layer_id);
                            self.status = format!("{} ({})", hit.name, hit.kind);
                        }
                    } else if resp.clicked() {
                        self.selected = None;
                    }
                }
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
    let mut min_lng = f64::INFINITY;
    let mut max_lng = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut any = false;
    for feat in features {
        if !visible(feat.layer_id.as_str()) {
            continue;
        }
        if !feat.lng.is_finite() || !feat.lat.is_finite() {
            continue;
        }
        any = true;
        min_lng = min_lng.min(feat.lng);
        max_lng = max_lng.max(feat.lng);
        min_lat = min_lat.min(feat.lat);
        max_lat = max_lat.max(feat.lat);
    }
    if !any {
        return None;
    }
    tiles::fit_view(min_lng, min_lat, max_lng, max_lat, width, height)
}

/// Headless BMP preview (24-bit, hand-rolled): same `project` math as the
/// canvas, XYZ tiles under per-layer colors. Lets a windowless session see
/// the map. `with_tiles=false` keeps unit tests offline; smoke passes true.
fn render_preview(
    features: &[store::StoredFeature],
    layers: &[LayerRow],
    width: u32,
    height: u32,
    with_tiles: bool,
    template: &str,
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
                let png = match tiles::fetch_tile_png_as(t, template) {
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
        // Stored color wins, else the palette (same rule as the canvas).
        let c = layer_draw_color(layers, feat.layer_id.as_str());
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
    let app = GisCraftApp::open_with_seed(None);
    let total_features: u64 = app.layers.iter().map(|l| l.features).sum();
    println!(
        "giscraft smoke: {} layers, {} features, db={}",
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
    preview_path.push(format!("giscraft-preview-{}.bmp", std::process::id()));
    match render_preview(
        &app.features,
        &app.layers,
        1280,
        860,
        true,
        &resolve_tile_template(&app.layers),
    ) {
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
    std::env::var("GISCRAFT_CONTROL_PORT")
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
                    "giscraft control listening on 127.0.0.1:{port} (no auth: local use only)"
                );
                std::thread::spawn(move || control::serve(listener, control_tx));
            }
            Err(e) => eprintln!("control bind failed (continuing without): {e}"),
        }
    }
    let control_rx = control_port.map(|_| control_rx);
    let viewport = egui::ViewportBuilder::default()
        .with_title("giscraft")
        .with_inner_size([1280.0, 860.0])
        .with_min_inner_size([900.0, 600.0]);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "giscraft",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(high_contrast_dark());
            Ok(Box::new(GisCraftApp::open_with_seed(control_rx)))
        }),
    );
    if let Err(e) = result {
        eprintln!("giscraft exited: {e}");
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
            source: "osm".to_string(),
            geom_type: "point".to_string(),
            visible: true,
            color: String::new(),
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
    fn stored_color_overrides_palette() {
        let plain = test_layers();
        let palette = layer_draw_color(&plain, "rx");
        assert_eq!(palette, layer_color("rx"));
        let mut styled = test_layers();
        if let Some(l) = styled.first_mut() {
            l.color = "#ff0000".to_string();
        }
        let custom = layer_draw_color(&styled, "rx");
        assert_eq!(custom, egui::Color32::from_rgb(255, 0, 0));
        assert_ne!(custom, palette);
        if let Some(l) = styled.first_mut() {
            l.color = "junk".to_string();
        }
        assert_eq!(layer_draw_color(&styled, "rx"), layer_color("rx"));
        assert_eq!(layer_draw_color(&styled, "missing"), layer_color("missing"));
    }

    #[test]
    fn render_preview_is_bmp_and_rejects_hostile_size() {
        assert!(render_preview(&test_features(), &test_layers(), 0, 600, false, "").is_err());
        assert!(render_preview(&test_features(), &test_layers(), 5000, 600, false, "").is_err());
        let bmp = render_preview(&test_features(), &test_layers(), 320, 200, false, "");
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
        let bmp = render_preview(&[feat], &test_layers(), 320, 200, false, "");
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

    fn test_app(features: Vec<store::StoredFeature>, visible: bool) -> GisCraftApp {
        GisCraftApp {
            db_path: ":memory:".to_string(),
            locale: giscraft_core::i18n::AppLocale::default(),
            layers: vec![LayerRow {
                id: "rx".to_string(),
                name: "Rx".to_string(),
                source: "osm".to_string(),
                geom_type: "point".to_string(),
                visible,
                color: String::new(),
                features: features.len() as u64,
            }],
            features,
            selected: None,
            status: String::new(),
            log: Vec::new(),
            zoom: 15.0,
            center_lng: 16.35,
            center_lat: 48.225,
            fit_requested: false,
            fit_only: None,
            confirm_new: false,
            show_about: false,
            show_help: false,
            show_settings: false,
            dialog: Dialog::None,
            toolbox_query: String::new(),
            pending: None,
            tile_template: String::new(),
            textures: HashMap::new(),
            control_rx: None,
            last_rect: None,
        }
    }

    fn center_feature(name: &str) -> store::StoredFeature {
        store::StoredFeature {
            layer_id: "rx".to_string(),
            name: name.to_string(),
            kind: "node".to_string(),
            lng: 16.35,
            lat: 48.225,
            geom_json: None,
        }
    }

    #[test]
    fn pick_at_finds_points_and_respects_visibility() {
        let app = test_app(vec![center_feature("A")], true);
        let rect = test_rect();
        let pa = project(
            16.35,
            48.225,
            app.center_lng,
            app.center_lat,
            app.zoom,
            &rect,
        );
        assert!((pa.x - 400.0).abs() < 1.0 && (pa.y - 300.0).abs() < 1.0);
        let hit = app.pick_at(pa, &rect);
        assert!(hit.is_some());
        assert_eq!(hit.map(|h| h.name), Some("A".to_string()));
        assert!(app.pick_at(egui::Pos2::new(790.0, 590.0), &rect).is_none());
        let hidden = test_app(vec![center_feature("A")], false);
        assert!(hidden.pick_at(pa, &rect).is_none());
    }

    #[test]
    fn pick_at_finds_polygon_interior() {
        let feat = geom_feature(
            r#"{"type":"Polygon","coordinates":[[[16.348,48.223],[16.352,48.223],[16.352,48.227],[16.348,48.227],[16.348,48.223]]]}"#,
        );
        let app = test_app(vec![feat], true);
        let rect = test_rect();
        let inside = project(
            16.35,
            48.225,
            app.center_lng,
            app.center_lat,
            app.zoom,
            &rect,
        );
        assert!(app.pick_at(inside, &rect).is_some());
        assert!(app.pick_at(egui::Pos2::new(10.0, 590.0), &rect).is_none());
    }
}
