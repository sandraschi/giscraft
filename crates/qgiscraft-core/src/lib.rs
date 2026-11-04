//! L1 GIS core: protocols, project model, command bus.
//!
//! UI runtimes (egui), transports (MCP stdio), and CLIs are forbidden
//! from touching geo state directly. Everything routes through
//! [`Command::dispatch`] with typed params, so the MCP server is a thin frame
//! around one async function.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod control;
pub mod error;
pub mod geo;
pub mod geocode;
pub mod import;
pub mod mapforge;
pub mod overpass;
pub mod processing;
pub mod project;
pub mod tiles;
pub mod triangulate;
pub mod wkt;

use error::GisError;
use serde_json::{json, Value};

/// Unique string IDs exposed over MCP `tools/list` and the CLI.
pub const CMD_STATUS: &str = "status";
pub const CMD_PROJECT_STATUS: &str = "project.status";
pub const CMD_LAYER_ADD: &str = "layer.add";
pub const CMD_LAYER_LIST: &str = "layer.list";
pub const CMD_LAYER_REMOVE: &str = "layer.remove";
pub const CMD_GEOCODE_SEARCH: &str = "geocode.search";
pub const CMD_OVERPASS_QUERY: &str = "overpass.query";
pub const CMD_FEATURES_LIST: &str = "features.list";
pub const CMD_LAYER_IMPORT: &str = "layer.import";
pub const CMD_PROCESSING_BUFFER: &str = "processing.buffer";
pub const CMD_MAP_FORGE: &str = "map.forge";
pub const CMD_WKT_SHOW: &str = "wkt.show";
pub const CMD_PROCESSING_GRID: &str = "processing.grid";
pub const CMD_PROCESSING_NEAREST: &str = "processing.nearest";
pub const CMD_PROCESSING_GEODESIC: &str = "processing.geodesic";
pub const CMD_LAYER_COLOR: &str = "layer.color";
pub const CMD_EXPORT_PNG: &str = "export.png";

/// Bounded command bus. New capability = new variant + dispatch arm + test.
#[derive(Debug, Clone)]
pub enum Command {
    Status,
    ProjectStatus,
    LayerAdd {
        id: String,
        name: String,
        source: String,
        geom_type: String,
        crs: Option<String>,
    },
    LayerList,
    LayerRemove {
        id: String,
    },
    GeocodeSearch {
        query: String,
        limit: Option<u8>,
        live: bool,
    },
    OverpassQuery {
        key: String,
        value: String,
        south: Option<f64>,
        west: Option<f64>,
        north: Option<f64>,
        east: Option<f64>,
        live: bool,
    },
    LayerImport {
        path: String,
        format: Option<String>,
        layer_id: Option<String>,
    },
    ProcessingBuffer {
        layer_id: String,
        distance_m: f64,
        segments: Option<usize>,
        output_layer: Option<String>,
    },
    MapForge {
        path: String,
        live: bool,
    },
    WktShow {
        wkt: String,
        layer_id: Option<String>,
    },
    ProcessingGrid {
        south: Option<f64>,
        west: Option<f64>,
        north: Option<f64>,
        east: Option<f64>,
        cell_size_m: Option<f64>,
        hex: bool,
        output_layer: Option<String>,
    },
    ProcessingNearest {
        source_layer: String,
        target_layer: String,
        output_layer: Option<String>,
    },
    ProcessingGeodesic {
        shape: String,
        lng: Option<f64>,
        lat: Option<f64>,
        radius_m: Option<f64>,
        major_m: Option<f64>,
        minor_m: Option<f64>,
        bearing_deg: Option<f64>,
        lng2: Option<f64>,
        lat2: Option<f64>,
        segments: Option<usize>,
        output_layer: Option<String>,
    },
    LayerColor {
        layer_id: String,
        color: Option<String>,
    },
    ExportPng {
        width: Option<u32>,
        height: Option<u32>,
    },
}

fn encode(op: &str, result: Result<Value, GisError>) -> Value {
    match result {
        Ok(value) => value,
        Err(e) => json!({"error": e.to_string(), "op": op}),
    }
}

/// Dispatch one command headlessly. Never panics; failures become JSON values.
pub async fn dispatch(cmd: Command) -> Value {
    match cmd {
        Command::Status => json!({
            "app": "qgiscraft-mcp",
            "version": env!("CARGO_PKG_VERSION"),
            "mode": "pure-rust",
            "track": "A-native-egui + B-web-parallel-11253/11254",
            "commands": [
                CMD_STATUS, CMD_PROJECT_STATUS,
                CMD_LAYER_ADD, CMD_LAYER_LIST, CMD_LAYER_REMOVE,
                CMD_GEOCODE_SEARCH, CMD_OVERPASS_QUERY, CMD_FEATURES_LIST, CMD_LAYER_IMPORT, CMD_PROCESSING_BUFFER, CMD_MAP_FORGE, CMD_WKT_SHOW,                 CMD_PROCESSING_GRID, CMD_PROCESSING_NEAREST, CMD_PROCESSING_GEODESIC, CMD_LAYER_COLOR, CMD_EXPORT_PNG,
            ],
            "agent_contract": "CLI-first: every command supports --json; `mcp` subcommand mirrors the same dispatch over stdio",
        }),
        Command::ProjectStatus => json!({
            "project": "wien-demo",
            "crs": "EPSG:4326",
            "layers": 0,
            "note": "Phase 0 scaffold: in-memory only, SQLite store lands via qgiscraft-store",
        }),
        Command::LayerAdd {
            id,
            name,
            source,
            geom_type,
            crs,
        } => {
            let mut p = project::Project::new("scratch".to_string());
            let layer = project::Layer {
                id: id.clone(),
                name: name.clone(),
                source,
                geom_type,
                crs: crs.unwrap_or_else(|| "EPSG:4326".to_string()),
                visible: true,
            };
            encode(
                "layer.add",
                p.add_layer(layer)
                    .map(|()| json!({"id": id, "name": name, "added": true})),
            )
        }
        Command::LayerList => {
            json!({"layers": [], "note": "Phase 0: SQLite listing via store crate"})
        }
        Command::LayerRemove { id } => {
            let mut p = project::Project::new("scratch".to_string());
            encode(
                "layer.remove",
                p.remove_layer(&id)
                    .map(|removed| json!({"id": id, "removed": removed})),
            )
        }
        Command::GeocodeSearch { query, limit, live } => {
            if query.trim().is_empty() {
                return json!({"error": "invalid input: query must not be empty", "op": "geocode.search"});
            }
            let lim = limit.unwrap_or(5);
            if lim == 0 || lim > 50 {
                return json!({"error": "invalid input: limit must be 1..50", "op": "geocode.search"});
            }
            if !live {
                json!({"query": query, "limit": lim, "results": [{"name": "Stephansplatz, Wien", "lng": 16.3738, "lat": 48.2082, "source": "demo"}], "live": false})
            } else {
                match geocode::photon_search(&query, lim).await {
                    Ok(hits) => {
                        json!({"query": query, "limit": lim, "results": hits, "live": true})
                    }
                    Err(e) => json!({"error": e.to_string(), "op": "geocode.search"}),
                }
            }
        }
        Command::OverpassQuery {
            key,
            value,
            south,
            west,
            north,
            east,
            live,
        } => {
            let bbox = match (south, west, north, east) {
                (Some(s), Some(w), Some(n), Some(e)) => match geo::Bbox::new(w, s, e, n) {
                    Ok(b) => Some(b),
                    Err(e) => return json!({"error": e.to_string(), "op": "overpass.query"}),
                },
                (None, None, None, None) => None,
                _ => {
                    return json!({"error": "invalid input: bbox needs all of south,west,north,east or none", "op": "overpass.query"});
                }
            };
            let ql = match overpass::build_tag_query(&key, &value, bbox) {
                Ok(ql) => ql,
                Err(e) => return json!({"error": e.to_string(), "op": "overpass.query"}),
            };
            if !live {
                return json!({"key": key, "value": value, "ql": ql, "live": false, "note": "dry-run: re-run with live=true to execute against overpass-api.de"});
            }
            match overpass::execute_ql(&ql).await {
                Ok(hits) => {
                    json!({"key": key, "value": value, "ql": ql, "count": hits.len(), "hits": hits, "live": true})
                }
                Err(e) => json!({"error": e.to_string(), "op": "overpass.query"}),
            }
        }
        Command::LayerImport {
            path,
            format,
            layer_id,
        } => {
            if path.trim().is_empty() {
                return json!({"error": "invalid input: path must not be empty", "op": "layer.import"});
            }
            let text = match tokio::fs::read_to_string(path.trim()).await {
                Ok(t) => t,
                Err(e) => {
                    return json!({"error": format!("read failed: {e}"), "op": "layer.import"});
                }
            };
            let fmt = format.as_deref().unwrap_or("auto").to_lowercase();
            let auto_ext = std::path::Path::new(path.trim())
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let kind = if fmt == "auto" {
                match auto_ext.as_str() {
                    "geojson" | "json" => "geojson",
                    "csv" | "txt" => "csv",
                    other => {
                        return json!({"error": format!("cannot detect format from .{other} (use geojson|csv)"), "op": "layer.import"});
                    }
                }
            } else if fmt == "geojson" || fmt == "csv" {
                fmt.as_str()
            } else {
                return json!({"error": "invalid input: format must be auto|geojson|csv", "op": "layer.import"});
            };
            let feats = match kind {
                "geojson" => import::parse_geojson(&text),
                _ => import::parse_csv(&text, None, None),
            };
            let feats = match feats {
                Ok(f) => f,
                Err(e) => return json!({"error": e.to_string(), "op": "layer.import"}),
            };
            let stem = std::path::Path::new(path.trim())
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("import")
                .to_string();
            let slug: String = layer_id.unwrap_or_else(|| {
                stem.chars()
                    .map(|c| {
                        if c.is_alphanumeric() {
                            c.to_ascii_lowercase()
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>()
                    .trim_matches('-')
                    .to_string()
            });
            let slug = if slug.is_empty() {
                "import".to_string()
            } else {
                slug
            };
            let hits: Vec<Value> = feats
                .iter()
                .map(|f| {
                    let mut h = serde_json::json!({
                        "name": f.name, "kind": f.kind, "lng": f.lng, "lat": f.lat,
                    });
                    if let Some(g) = f.geom_json.as_ref() {
                        if let Ok(gv) = serde_json::from_str::<Value>(g) {
                            h["geom"] = gv;
                        }
                    }
                    h
                })
                .collect();
            json!({"layer": slug, "name": stem, "count": hits.len(), "hits": hits})
        }
        Command::ProcessingBuffer {
            layer_id,
            distance_m,
            segments,
            output_layer,
        } => {
            if layer_id.trim().is_empty() {
                return json!({"error": "invalid input: layer_id must not be empty", "op": "processing.buffer"});
            }
            if distance_m <= 0.0 || !distance_m.is_finite() || distance_m > 1_000_000.0 {
                return json!({"error": "invalid input: distance_m must be 0..1000000", "op": "processing.buffer"});
            }
            let segs = segments.unwrap_or(24).clamp(8, 256);
            let out_id = output_layer.unwrap_or_else(|| format!("{layer_id}-buffer"));
            json!({
                "op": "processing.buffer",
                "layer_id": layer_id,
                "distance_m": distance_m,
                "segments": segs,
                "output_layer": out_id,
                "note": "Core validation ok: persistence performed by L2 store or app control handler"
            })
        }
        Command::MapForge { path, live } => {
            if path.trim().is_empty() {
                return json!({"error": "invalid input: path must not be empty", "op": "map.forge"});
            }
            let text = match tokio::fs::read_to_string(path.trim()).await {
                Ok(t) => t,
                Err(e) => {
                    return json!({"error": format!("read failed: {e}"), "op": "map.forge"});
                }
            };
            let spec = match mapforge::parse_spec(&text) {
                Ok(s) => s,
                Err(e) => return json!({"error": e.to_string(), "op": "map.forge"}),
            };
            let plan = match mapforge::compile_plan(&spec) {
                Ok(p) => p,
                Err(e) => return json!({"error": e.to_string(), "op": "map.forge"}),
            };
            if !live {
                return json!({
                    "op": "map.forge",
                    "slug": plan.slug,
                    "title": plan.title,
                    "bbox": plan.bbox,
                    "layers": plan.layers,
                    "live": false,
                    "note": "dry-run: re-run with live=true to execute against overpass-api.de"
                });
            }
            let mut executed: Vec<Value> = Vec::new();
            for item in &plan.layers {
                match overpass::execute_ql(&item.ql).await {
                    Ok(hits) => {
                        let count = hits.len();
                        executed.push(json!({
                            "id": item.id,
                            "name": item.name,
                            "key": item.key,
                            "value": item.value,
                            "count": count,
                            "hits": hits,
                        }));
                    }
                    Err(e) => {
                        return json!({"error": e.to_string(), "op": "map.forge"});
                    }
                }
            }
            json!({
                "op": "map.forge",
                "slug": plan.slug,
                "title": plan.title,
                "bbox": plan.bbox,
                "layers": executed,
                "live": true,
            })
        }
        Command::WktShow { wkt, layer_id } => {
            if wkt.trim().is_empty() {
                return json!({"error": "invalid input: wkt must not be empty", "op": "wkt.show"});
            }
            if wkt.chars().count() > 1_000_000 {
                return json!({"error": "invalid input: wkt exceeds 1000000 chars", "op": "wkt.show"});
            }
            let feats = match wkt::parse_wkt(&wkt) {
                Ok(f) => f,
                Err(e) => return json!({"error": e.to_string(), "op": "wkt.show"}),
            };
            let layer = match layer_id {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => "wkt".to_string(),
            };
            if layer.chars().count() > 64 {
                return json!({"error": "invalid input: layer id exceeds 64 chars", "op": "wkt.show"});
            }
            let hits: Vec<Value> = feats
                .iter()
                .map(|f| {
                    let geom: Value = serde_json::from_str(&f.geojson).unwrap_or(Value::Null);
                    json!({"name": f.name, "kind": f.kind, "lng": f.lng, "lat": f.lat, "geom": geom})
                })
                .collect();
            json!({
                "op": "wkt.show",
                "layer": layer.clone(),
                "name": layer,
                "count": hits.len(),
                "hits": hits,
            })
        }
        Command::ProcessingGrid {
            south,
            west,
            north,
            east,
            cell_size_m,
            hex,
            output_layer,
        } => {
            let (s, w, n, e) = match (south, west, north, east) {
                (Some(s), Some(w), Some(n), Some(e)) => (s, w, n, e),
                _ => {
                    return json!({"error": "invalid input: grid needs south/west/north/east", "op": "processing.grid"});
                }
            };
            let feats =
                match processing::grid_features(s, w, n, e, cell_size_m.unwrap_or(500.0), hex) {
                    Ok(f) => f,
                    Err(e) => return json!({"error": e.to_string(), "op": "processing.grid"}),
                };
            let out_id = match output_layer {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => {
                    if hex {
                        "grid-hex".to_string()
                    } else {
                        "grid".to_string()
                    }
                }
            };
            let hits: Vec<Value> = feats
                .iter()
                .map(|f| {
                    let geom: Value = serde_json::from_str(f.geom_json.as_deref().unwrap_or("null"))
                        .unwrap_or(Value::Null);
                    json!({"name": f.name, "kind": f.kind, "lng": f.lng, "lat": f.lat, "geom": geom})
                })
                .collect();
            json!({
                "op": "processing.grid",
                "layer": out_id,
                "name": out_id,
                "count": hits.len(),
                "hits": hits,
            })
        }
        Command::ProcessingNearest {
            source_layer,
            target_layer,
            output_layer,
        } => {
            if source_layer.trim().is_empty() || target_layer.trim().is_empty() {
                return json!({"error": "invalid input: source_layer and target_layer must not be empty", "op": "processing.nearest"});
            }
            let out_id = match output_layer {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => format!("{source_layer}-nearest"),
            };
            json!({
                "op": "processing.nearest",
                "source_layer": source_layer,
                "target_layer": target_layer,
                "output_layer": out_id,
                "note": "Core validation ok: persistence performed by L2 store or app control handler"
            })
        }
        Command::ProcessingGeodesic {
            shape,
            lng,
            lat,
            radius_m,
            major_m,
            minor_m,
            bearing_deg,
            lng2,
            lat2,
            segments,
            output_layer,
        } => {
            let shape = shape.trim().to_ascii_lowercase();
            if shape != "circle" && shape != "ellipse" && shape != "line" {
                return json!({"error": "invalid input: shape must be circle, ellipse or line", "op": "processing.geodesic"});
            }
            let (c_lng, c_lat) = match (lng, lat) {
                (Some(x), Some(y)) => (x, y),
                _ => {
                    return json!({"error": "invalid input: geodesic needs lng/lat center", "op": "processing.geodesic"});
                }
            };
            let segs = segments.unwrap_or(64);
            let ring = if shape == "circle" {
                match radius_m {
                    Some(r) => match processing::geodesic_circle(c_lng, c_lat, r, segs) {
                        Ok(ring) => ring,
                        Err(e) => {
                            return json!({"error": e.to_string(), "op": "processing.geodesic"})
                        }
                    },
                    None => {
                        return json!({"error": "invalid input: circle needs radius_m", "op": "processing.geodesic"});
                    }
                }
            } else if shape == "ellipse" {
                match (major_m, minor_m) {
                    (Some(a), Some(b)) => {
                        match processing::geodesic_ellipse(
                            c_lng,
                            c_lat,
                            a,
                            b,
                            bearing_deg.unwrap_or(0.0),
                            segs,
                        ) {
                            Ok(ring) => ring,
                            Err(e) => {
                                return json!({"error": e.to_string(), "op": "processing.geodesic"})
                            }
                        }
                    }
                    _ => {
                        return json!({"error": "invalid input: ellipse needs major_m and minor_m", "op": "processing.geodesic"});
                    }
                }
            } else {
                match (lng2, lat2) {
                    (Some(x2), Some(y2)) => match processing::xy_to_line(c_lng, c_lat, x2, y2) {
                        Ok(pts) => pts,
                        Err(e) => {
                            return json!({"error": e.to_string(), "op": "processing.geodesic"})
                        }
                    },
                    _ => {
                        return json!({"error": "invalid input: line needs lng2/lat2", "op": "processing.geodesic"});
                    }
                }
            };
            let closed: Vec<(f64, f64)> = if shape == "line" {
                ring.clone()
            } else {
                let mut c = ring.clone();
                if let Some(first) = ring.first().copied() {
                    if c.last() != Some(&first) {
                        c.push(first);
                    }
                }
                c
            };
            let inner: Vec<String> = closed.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
            let geom_kind = if shape == "line" {
                "LineString"
            } else {
                "Polygon"
            };
            let coords = if shape == "line" {
                format!("[{}]", inner.join(","))
            } else {
                format!("[[{}]]", inner.join(","))
            };
            let geojson = format!("{{\"type\":\"{geom_kind}\",\"coordinates\":{coords}}}");
            let out_id = match output_layer {
                Some(id) if !id.trim().is_empty() => id.trim().to_string(),
                _ => format!("geodesic-{shape}"),
            };
            let (cx, cy) = (c_lng, c_lat);
            json!({
                "op": "processing.geodesic",
                "layer": out_id,
                "name": out_id,
                "count": 1,
                "hits": [{"name": out_id, "kind": "way", "lng": cx, "lat": cy, "geom": serde_json::from_str::<Value>(&geojson).unwrap_or(Value::Null)}],
            })
        }
        Command::LayerColor { layer_id, color } => {
            if layer_id.trim().is_empty() {
                return json!({"error": "invalid input: layer_id must not be empty", "op": "layer.color"});
            }
            let want = color.unwrap_or_default();
            if tiles::parse_layer_color(&want).is_err() {
                return json!({"error": "invalid input: color must be #rrggbb or empty", "op": "layer.color"});
            }
            json!({
                "op": "layer.color",
                "layer_id": layer_id,
                "color": want,
                "note": "Core validation ok: persistence performed by L2 store or app control handler"
            })
        }
        Command::ExportPng {
            width: _,
            height: _,
        } => {
            json!({"error": "unsupported: export.png render path lands Phase 1 (vello_cpu)", "op": "export.png"})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn status_lists_all_commands() {
        let v = dispatch(Command::Status).await;
        let cmds = v
            .get("commands")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(cmds.len(), 17);
    }

    #[tokio::test]
    async fn layer_color_validates() {
        let v = dispatch(Command::LayerColor {
            layer_id: "pharms".to_string(),
            color: Some("#ff0000".to_string()),
        })
        .await;
        assert_eq!(v.get("color").and_then(|s| s.as_str()), Some("#ff0000"));
        let bad = dispatch(Command::LayerColor {
            layer_id: "pharms".to_string(),
            color: Some("red".to_string()),
        })
        .await;
        assert!(bad.get("error").is_some());
        let empty = dispatch(Command::LayerColor {
            layer_id: " ".to_string(),
            color: None,
        })
        .await;
        assert!(empty.get("error").is_some());
    }

    #[tokio::test]
    async fn processing_geodesic_circle_and_errors() {
        let v = dispatch(Command::ProcessingGeodesic {
            shape: "circle".to_string(),
            lng: Some(16.3738),
            lat: Some(48.2082),
            radius_m: Some(1000.0),
            major_m: None,
            minor_m: None,
            bearing_deg: None,
            lng2: None,
            lat2: None,
            segments: Some(24),
            output_layer: None,
        })
        .await;
        assert_eq!(v.get("count").and_then(|n| n.as_u64()), Some(1));
        assert_eq!(
            v.get("layer").and_then(|s| s.as_str()),
            Some("geodesic-circle")
        );
        let bad_shape = dispatch(Command::ProcessingGeodesic {
            shape: "square".to_string(),
            lng: Some(16.3),
            lat: Some(48.2),
            radius_m: Some(100.0),
            major_m: None,
            minor_m: None,
            bearing_deg: None,
            lng2: None,
            lat2: None,
            segments: None,
            output_layer: None,
        })
        .await;
        assert!(bad_shape.get("error").is_some());
        let no_radius = dispatch(Command::ProcessingGeodesic {
            shape: "circle".to_string(),
            lng: Some(16.3),
            lat: Some(48.2),
            radius_m: None,
            major_m: None,
            minor_m: None,
            bearing_deg: None,
            lng2: None,
            lat2: None,
            segments: None,
            output_layer: None,
        })
        .await;
        assert!(no_radius.get("error").is_some());
    }

    #[tokio::test]
    async fn processing_buffer_validates_layer_and_distance() {
        let empty = dispatch(Command::ProcessingBuffer {
            layer_id: " ".to_string(),
            distance_m: 100.0,
            segments: None,
            output_layer: None,
        })
        .await;
        assert!(empty.get("error").is_some());

        let bad_dist = dispatch(Command::ProcessingBuffer {
            layer_id: "test".to_string(),
            distance_m: -50.0,
            segments: None,
            output_layer: None,
        })
        .await;
        assert!(bad_dist.get("error").is_some());

        let ok = dispatch(Command::ProcessingBuffer {
            layer_id: "poi".to_string(),
            distance_m: 250.0,
            segments: Some(16),
            output_layer: Some("poi-buf".to_string()),
        })
        .await;
        assert_eq!(
            ok.get("output_layer").and_then(|v| v.as_str()),
            Some("poi-buf")
        );
    }

    #[tokio::test]
    async fn hostile_empty_layer_id_becomes_error_value_not_panic() {
        let v = dispatch(Command::LayerAdd {
            id: String::new(),
            name: "x".to_string(),
            source: "osm".to_string(),
            geom_type: "point".to_string(),
            crs: None,
        })
        .await;
        assert!(v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .contains("invalid input"));
    }

    #[tokio::test]
    async fn geocode_demo_round_trip() {
        let v = dispatch(Command::GeocodeSearch {
            query: "Stephansplatz".to_string(),
            limit: None,
            live: false,
        })
        .await;
        assert_eq!(v.get("live").and_then(|b| b.as_bool()), Some(false));
    }

    #[tokio::test]
    async fn import_geojson_file_roundtrip() {
        let mut p = std::env::temp_dir();
        p.push(format!("qgis-test-import-{}.geojson", std::process::id()));
        let doc = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"name":"A"},"geometry":{"type":"Point","coordinates":[16.3,48.2]}},{"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[16.3,48.2],[16.4,48.25]]}}]}"#;
        if std::fs::write(&p, doc).is_err() {
            return;
        }
        let v = dispatch(Command::LayerImport {
            path: p.to_string_lossy().to_string(),
            format: None,
            layer_id: None,
        })
        .await;
        let _ = std::fs::remove_file(&p);
        assert_eq!(v.get("count").and_then(|n| n.as_u64()), Some(2));
        assert!(v.get("layer").and_then(|s| s.as_str()).is_some());
    }

    #[tokio::test]
    async fn import_missing_file_is_typed_error() {
        let v = dispatch(Command::LayerImport {
            path: "definitely-not-here-xyz.geojson".to_string(),
            format: None,
            layer_id: None,
        })
        .await;
        assert!(v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .contains("read failed"));
    }

    #[tokio::test]
    async fn mapforge_dry_run_compiles_ql_offline() {
        let mut p = std::env::temp_dir();
        p.push(format!("qgis-test-forge-{}.json", std::process::id()));
        let doc = r#"{"title":"Innere Stadt pharmacies","slug":"innere-stadt-pharmacies","bbox":{"south":48.195,"west":16.355,"north":48.225,"east":16.395},"layers":[{"id":"pharmacies","key":"amenity","value":"pharmacy"}]}"#;
        if std::fs::write(&p, doc).is_err() {
            return;
        }
        let v = dispatch(Command::MapForge {
            path: p.to_string_lossy().to_string(),
            live: false,
        })
        .await;
        let _ = std::fs::remove_file(&p);
        assert_eq!(v.get("live").and_then(|b| b.as_bool()), Some(false));
        assert_eq!(
            v.get("slug").and_then(|s| s.as_str()),
            Some("innere-stadt-pharmacies")
        );
        assert_eq!(
            v.get("layers").and_then(|l| l.as_array()).map(|a| a.len()),
            Some(1)
        );
    }

    #[tokio::test]
    async fn wkt_show_parses_inline_no_file_no_network() {
        let v = dispatch(Command::WktShow {
            wkt: "POINT (16.3738 48.2082)".to_string(),
            layer_id: None,
        })
        .await;
        assert_eq!(v.get("count").and_then(|n| n.as_u64()), Some(1));
        assert_eq!(v.get("layer").and_then(|s| s.as_str()), Some("wkt"));
        let line = dispatch(Command::WktShow {
            wkt: "LINESTRING (16.3 48.2, 16.4 48.25)".to_string(),
            layer_id: Some("route".to_string()),
        })
        .await;
        assert_eq!(line.get("layer").and_then(|s| s.as_str()), Some("route"));
        let bad = dispatch(Command::WktShow {
            wkt: "POINT (16.3)".to_string(),
            layer_id: None,
        })
        .await;
        assert!(bad.get("error").is_some());
        let empty = dispatch(Command::WktShow {
            wkt: "   ".to_string(),
            layer_id: None,
        })
        .await;
        assert!(empty.get("error").is_some());
    }

    #[tokio::test]
    async fn processing_grid_computes_offline() {
        let v = dispatch(Command::ProcessingGrid {
            south: Some(48.20),
            west: Some(16.36),
            north: Some(48.21),
            east: Some(16.37),
            cell_size_m: Some(500.0),
            hex: false,
            output_layer: None,
        })
        .await;
        assert_eq!(v.get("count").and_then(|n| n.as_u64()), Some(6));
        assert_eq!(v.get("layer").and_then(|s| s.as_str()), Some("grid"));
        let missing = dispatch(Command::ProcessingGrid {
            south: None,
            west: None,
            north: None,
            east: None,
            cell_size_m: None,
            hex: true,
            output_layer: None,
        })
        .await;
        assert!(missing.get("error").is_some());
    }

    #[tokio::test]
    async fn processing_nearest_validates_layers() {
        let v = dispatch(Command::ProcessingNearest {
            source_layer: "bikes".to_string(),
            target_layer: "pharms".to_string(),
            output_layer: None,
        })
        .await;
        assert_eq!(
            v.get("output_layer").and_then(|s| s.as_str()),
            Some("bikes-nearest")
        );
        let empty = dispatch(Command::ProcessingNearest {
            source_layer: " ".to_string(),
            target_layer: "pharms".to_string(),
            output_layer: None,
        })
        .await;
        assert!(empty.get("error").is_some());
    }

    #[tokio::test]
    async fn mapforge_missing_file_is_typed_error() {
        let v = dispatch(Command::MapForge {
            path: "definitely-not-here-forge-xyz.json".to_string(),
            live: false,
        })
        .await;
        assert!(v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .contains("read failed"));
    }

    #[test]
    fn dispatch_future_is_send() {
        fn assert_send<T: Send>(_: T) {}
        assert_send(dispatch(Command::Status));
    }
}
