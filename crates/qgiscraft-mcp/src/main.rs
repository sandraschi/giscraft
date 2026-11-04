//! Stdout is pure JSON-RPC. Every log line goes to stderr via `eprintln!`.
//! The MCP layer never touches geo state; it dispatches `Command` IDs
//! into `qgiscraft-core` and frames the resulting `serde_json::Value`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::{Parser, Subcommand};
use qgiscraft_core::control;
use qgiscraft_core::{dispatch, Command};
use qgiscraft_store as store;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(
    name = "qgiscraft-mcp",
    about = "QGIS-class GIS: 1:1 clone shell, modern Rust backend (CLI-first, MCP over stdio)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show app status (human or --json for agents)
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Project status (CRS, layer count)
    ProjectStatus {
        #[arg(long)]
        json: bool,
    },
    /// Add a layer (id/name/source/geom-type; persists to SQLite)
    LayerAdd {
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "osm")]
        source: String,
        #[arg(long, default_value = "point")]
        geom_type: String,
        #[arg(long)]
        crs: Option<String>,
        #[arg(long, num_args(0..=1), default_value = "true", default_missing_value = "true")]
        visible: bool,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List layers (SQLite project DB)
    LayerList {
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Remove a layer by id (SQLite project DB)
    LayerRemove {
        #[arg(long)]
        id: String,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Set a layer's display color (#rrggbb, empty clears to palette)
    LayerColor {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "")]
        color: String,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Geocode search (demo default: Vienna; --live hits Photon, keyless)
    GeocodeSearch {
        #[arg(long)]
        query: String,
        #[arg(long)]
        limit: Option<u8>,
        #[arg(long)]
        live: bool,
        #[arg(long)]
        json: bool,
    },
    /// Overpass tag search (dry-run builds QL; --live executes, keyless)
    OverpassQuery {
        #[arg(long, default_value = "amenity")]
        key: String,
        #[arg(long, default_value = "pharmacy")]
        value: String,
        #[arg(long)]
        south: Option<f64>,
        #[arg(long)]
        west: Option<f64>,
        #[arg(long)]
        north: Option<f64>,
        #[arg(long)]
        east: Option<f64>,
        #[arg(long)]
        live: bool,
        #[arg(long)]
        ingest: bool,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List ingested point features (SQLite project DB)
    FeaturesList {
        #[arg(long)]
        layer_id: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Import a GeoJSON/CSV file as a layer (format auto-detected or explicit)
    LayerImport {
        #[arg(long)]
        path: String,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        layer_id: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Buffer point features of a layer into circle polygons (persists to SQLite)
    ProcessingBuffer {
        #[arg(long)]
        layer_id: String,
        #[arg(long, default_value = "200.0")]
        distance: f64,
        #[arg(long)]
        segments: Option<usize>,
        #[arg(long)]
        output_layer: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Build a rect or hex grid over a bbox (persists to SQLite)
    ProcessingGrid {
        #[arg(long)]
        south: f64,
        #[arg(long)]
        west: f64,
        #[arg(long)]
        north: f64,
        #[arg(long)]
        east: f64,
        #[arg(long, default_value = "500.0")]
        cell_size: f64,
        #[arg(long)]
        hex: bool,
        #[arg(long)]
        output_layer: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Nearest-neighbor join: hub lines from source to closest target (persists to SQLite)
    ProcessingNearest {
        #[arg(long)]
        source_layer: String,
        #[arg(long)]
        target_layer: String,
        #[arg(long)]
        output_layer: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// True-distance geodesic shape: circle, ellipse or line (persists to SQLite)
    ProcessingGeodesic {
        #[arg(long)]
        shape: String,
        #[arg(long)]
        lng: Option<f64>,
        #[arg(long)]
        lat: Option<f64>,
        #[arg(long)]
        radius_m: Option<f64>,
        #[arg(long)]
        major_m: Option<f64>,
        #[arg(long)]
        minor_m: Option<f64>,
        #[arg(long)]
        bearing_deg: Option<f64>,
        #[arg(long)]
        lng2: Option<f64>,
        #[arg(long)]
        lat2: Option<f64>,
        #[arg(long)]
        segments: Option<usize>,
        #[arg(long)]
        output_layer: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Forge a map from a MapSpec JSON file (dry-run builds QL; --live executes, keyless)
    ForgeFromSpec {
        #[arg(long)]
        path: String,
        #[arg(long)]
        live: bool,
        #[arg(long)]
        ingest: bool,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show WKT geometry as a layer (persists to db)
    WktShow {
        #[arg(long)]
        wkt: String,
        #[arg(long)]
        layer_id: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Doppler geolocation: rank grid candidates vs ADS-B tracks (persists to db)
    DopplerLocate {
        #[arg(long)]
        path: String,
        #[arg(long)]
        south: Option<f64>,
        #[arg(long)]
        west: Option<f64>,
        #[arg(long)]
        north: Option<f64>,
        #[arg(long)]
        east: Option<f64>,
        #[arg(long)]
        step_deg: Option<f64>,
        #[arg(long)]
        top_n: Option<usize>,
        #[arg(long)]
        terrain_m: Option<f64>,
        #[arg(long)]
        output_layer: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Run the MCP server over stdio (logs on stderr, JSON-RPC on stdout).
    /// With --connect PORT, the DB-touching tools run in the live app.
    Mcp {
        #[arg(long)]
        connect: Option<u16>,
    },
}

/// One shared runtime. A failed build is a fatal startup error (exit 1).
fn rt() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| match Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("failed to start async runtime: {e}");
            std::process::exit(1);
        }
    })
}

fn run(cmd: Command) -> Value {
    rt().block_on(dispatch(cmd))
}

/// L2 persistence owner: core dispatch validates, SQLite stores.
/// `db=None` resolves to `$QGISCRAFT_DB` or `./qgiscraft.db`.
/// Forward one tool call to the live app over the control channel.
/// Returns the app's `result` verbatim, or a typed error object.
fn forward_control(port: u16, method: &str, params: Value) -> Value {
    match control::roundtrip(port, &Value::Null, method, &params) {
        Ok(reply) => match reply.get("ok").and_then(|v| v.as_bool()) {
            Some(true) => reply.get("result").cloned().unwrap_or(Value::Null),
            _ => {
                let err = reply
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("control error");
                serde_json::json!({"error": err.to_string(), "op": method})
            }
        },
        Err(e) => serde_json::json!({"error": e, "op": method}),
    }
}

fn layer_add(
    db: Option<&str>,
    id: &str,
    name: &str,
    source: Option<String>,
    geom_type: Option<String>,
    crs: Option<String>,
    visible: Option<bool>,
) -> Value {
    if id.trim().is_empty() || name.trim().is_empty() {
        return json!({"error": "invalid input: id and name must not be empty", "op": "layer.add"});
    }
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "layer.add"}),
    };
    let mut row = store::StoredLayer::new(id.to_string(), name.to_string(), source, geom_type, crs);
    row.visible = visible.unwrap_or(true);
    match store::add_layer(&conn, &row) {
        Ok(()) => {
            json!({"id": row.id, "name": row.name, "source": row.source, "geom_type": row.geom_type, "crs": row.crs, "visible": row.visible, "added": true})
        }
        Err(e) => json!({"error": e.to_string(), "op": "layer.add"}),
    }
}

fn layer_list(db: Option<&str>) -> Value {
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "layer.list"}),
    };
    let layers = match store::list_layers(&conn) {
        Ok(layers) => layers,
        Err(e) => return json!({"error": e.to_string(), "op": "layer.list"}),
    };
    let counts = store::feature_counts(&conn).unwrap_or_default();
    let rows: Vec<Value> = layers
        .into_iter()
        .map(|l| {
            let n = counts
                .iter()
                .find(|(id, _)| *id == l.id)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            json!({"id": l.id, "name": l.name, "source": l.source, "geom_type": l.geom_type, "crs": l.crs, "visible": l.visible, "color": l.color, "features": n})
        })
        .collect();
    json!({"count": rows.len(), "layers": rows})
}

fn layer_remove(db: Option<&str>, id: &str) -> Value {
    if id.trim().is_empty() {
        return json!({"error": "invalid input: id must not be empty", "op": "layer.remove"});
    }
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "layer.remove"}),
    };
    match store::remove_layer(&conn, id) {
        Ok(removed) => json!({"id": id, "removed": removed}),
        Err(e) => json!({"error": e.to_string(), "op": "layer.remove"}),
    }
}

fn layer_color_set(db: Option<&str>, id: &str, color: &str) -> Value {
    let out = run(Command::LayerColor {
        layer_id: id.to_string(),
        color: Some(color.to_string()),
    });
    if out.get("error").is_some() {
        return out;
    }
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "layer.color"}),
    };
    match store::set_color(&conn, id, color.trim()) {
        Ok(updated) => json!({"id": id, "color": color.trim(), "updated": updated}),
        Err(e) => json!({"error": e.to_string(), "op": "layer.color"}),
    }
}

/// Sniff the layer geometry type from ingested hits: any Polygon wins,
/// else any LineString, else points. Shared by layer.import and wkt.show.
fn infer_geom_type(hits: &Value) -> &'static str {
    hits.as_array()
        .map(|arr| {
            let mut kind = "point";
            for h in arr {
                if let Some(t) = h
                    .get("geom")
                    .and_then(|g| g.get("type"))
                    .and_then(|t| t.as_str())
                {
                    if t == "Polygon" {
                        kind = "polygon";
                        break;
                    }
                    if t == "LineString" {
                        kind = "line";
                    }
                }
            }
            kind
        })
        .unwrap_or("point")
}

/// Ingest dispatch-result hits into the project DB under `layer_id`.
/// Pure-DB seam: tested with temp DBs, no network.
fn ingest_hits(
    db: Option<&str>,
    layer_id: &str,
    layer_name: &str,
    source: &str,
    geom_type: &str,
    hits: &Value,
    op: &str,
) -> Value {
    if layer_id.trim().is_empty() {
        return serde_json::json!({"error": "invalid input: layer id must not be empty", "op": op});
    }
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return serde_json::json!({"error": e.to_string(), "op": op}),
    };
    let arr = hits.as_array();
    let mut feats = Vec::new();
    if let Some(items) = arr {
        for h in items {
            let name = h
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("(unnamed)")
                .to_string();
            let kind = h
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("node")
                .to_string();
            let (lng, lat) = match (
                h.get("lng").and_then(|v| v.as_f64()),
                h.get("lat").and_then(|v| v.as_f64()),
            ) {
                (Some(lng), Some(lat)) => (lng, lat),
                _ => continue,
            };
            feats.push(store::StoredFeature {
                layer_id: layer_id.to_string(),
                name,
                kind,
                lng,
                lat,
                geom_json: h
                    .get("geom")
                    .filter(|g| !g.is_null())
                    .map(|g| g.to_string()),
            });
        }
    }
    let layer = store::StoredLayer::new(
        layer_id.to_string(),
        layer_name.to_string(),
        Some(source.to_string()),
        Some(geom_type.to_string()),
        None,
    );
    if let Err(e) = store::add_layer(&conn, &layer) {
        return serde_json::json!({"error": e.to_string(), "op": op});
    }
    match store::replace_features(&conn, layer_id, &feats) {
        Ok(n) => serde_json::json!({"layer": layer_id, "ingested": n}),
        Err(e) => serde_json::json!({"error": e.to_string(), "op": op}),
    }
}

fn features_list(db: Option<&str>, layer_id: Option<&str>) -> Value {
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "features.list"}),
    };
    match store::list_features(&conn, layer_id) {
        Ok(rows) => json!({"count": rows.len(), "features": rows}),
        Err(e) => json!({"error": e.to_string(), "op": "features.list"}),
    }
}

fn layer_buffer(
    db: Option<&str>,
    layer_id: &str,
    distance_m: f64,
    segments: Option<usize>,
    output_layer: Option<&str>,
) -> Value {
    if layer_id.trim().is_empty() {
        return json!({"error": "invalid input: layer_id must not be empty", "op": "processing.buffer"});
    }
    if distance_m <= 0.0 || !distance_m.is_finite() || distance_m > 1_000_000.0 {
        return json!({"error": "invalid input: distance must be 0..1000000 m", "op": "processing.buffer"});
    }
    let conn = match store::open_at(db) {
        Ok(c) => c,
        Err(e) => return json!({"error": e.to_string(), "op": "processing.buffer"}),
    };
    let feats = match store::list_features(&conn, Some(layer_id)) {
        Ok(f) => f,
        Err(e) => return json!({"error": e.to_string(), "op": "processing.buffer"}),
    };
    if feats.is_empty() {
        return json!({"error": format!("layer not found or empty: {layer_id}"), "op": "processing.buffer"});
    }
    let raw: Vec<qgiscraft_core::import::RawFeature> = feats
        .into_iter()
        .map(|f| qgiscraft_core::import::RawFeature {
            name: f.name,
            kind: f.kind,
            lng: f.lng,
            lat: f.lat,
            geom_json: f.geom_json,
        })
        .collect();
    let buffered =
        match qgiscraft_core::processing::buffer_features(&raw, distance_m, segments.unwrap_or(24))
        {
            Ok(b) => b,
            Err(e) => return json!({"error": e.to_string(), "op": "processing.buffer"}),
        };
    let out_id = output_layer
        .unwrap_or(&format!("{layer_id}-buffer"))
        .to_string();
    let out_name = format!("{layer_id} buffer ({distance_m}m)");
    let new_layer = store::StoredLayer::new(
        out_id.clone(),
        out_name,
        Some("processing:buffer".to_string()),
        Some("polygon".to_string()),
        None,
    );
    if let Err(e) = store::add_layer(&conn, &new_layer) {
        return json!({"error": e.to_string(), "op": "processing.buffer"});
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
        Ok(n) => json!({
            "input_layer": layer_id,
            "output_layer": out_id,
            "distance_m": distance_m,
            "buffered": n,
        }),
        Err(e) => json!({"error": e.to_string(), "op": "processing.buffer"}),
    }
}

fn print_json(value: &Value) {
    if let Ok(text) = serde_json::to_string_pretty(value) {
        println!("{text}");
    } else {
        println!("{{\"error\":\"encode output\"}}");
    }
}

fn show(value: Value, as_json: bool, header: &str) {
    if as_json {
        print_json(&value);
    } else {
        println!("{header}");
        print_json(&value);
    }
}

/// Pure, testable tool catalog. Mirrors `tools/list` without I/O.
fn tools_list() -> Value {
    json!({
        "tools": [
            {"name": "status", "description": "Show app status (mode, commands, contract)", "inputSchema": {"type": "object", "properties": {}}},
            {"name": "project.status", "description": "Project CRS and layer count", "inputSchema": {"type": "object", "properties": {}}},
            {"name": "layer.add", "description": "Add a layer: id, name required; source, geom_type, crs, visible, db optional", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "name": {"type": "string"}, "source": {"type": "string"}, "geom_type": {"type": "string"}, "crs": {"type": "string"}, "visible": {"type": "boolean"}, "db": {"type": "string"}}, "required": ["id", "name"]}},
            {"name": "layer.list", "description": "List layers in draw order (SQLite project DB; db optional)", "inputSchema": {"type": "object", "properties": {"db": {"type": "string"}}}},
            {"name": "layer.remove", "description": "Remove a layer by id (db optional)", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "db": {"type": "string"}}, "required": ["id"]}},
            {"name": "layer.color", "description": "Set a layer display color: id required, color #rrggbb (empty clears to palette), db optional", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "color": {"type": "string"}, "db": {"type": "string"}}, "required": ["id"]}},
            {"name": "geocode.search", "description": "Search places: query required, limit 1..50, live=false demo else Photon", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "limit": {"type": "integer"}, "live": {"type": "boolean"}}, "required": ["query"]}},
            {"name": "overpass.query", "description": "Overpass tag search: key/value required, bbox south/west/north/east optional, live=false dry-runs QL else executes; ingest=true writes hits to db", "inputSchema": {"type": "object", "properties": {"key": {"type": "string"}, "value": {"type": "string"}, "south": {"type": "number"}, "west": {"type": "number"}, "north": {"type": "number"}, "east": {"type": "number"}, "live": {"type": "boolean"}, "ingest": {"type": "boolean"}, "db": {"type": "string"}}, "required": ["key", "value"]}},
            {"name": "features.list", "description": "List ingested point features, optionally filtered to one layer (db optional)", "inputSchema": {"type": "object", "properties": {"layer_id": {"type": "string"}, "db": {"type": "string"}}}},
            {"name": "layer.import", "description": "Import a GeoJSON/CSV file as a layer: format auto-detected by extension or explicit (geojson|csv); layer_id defaults to the file stem; persists to db", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "format": {"type": "string"}, "layer_id": {"type": "string"}, "db": {"type": "string"}}, "required": ["path"]}},
            {"name": "processing.buffer", "description": "Buffer point features of a layer into circle polygons: layer_id required; distance (meters, default 200.0), segments (8..256, default 24), output_layer, db optional", "inputSchema": {"type": "object", "properties": {"layer_id": {"type": "string"}, "distance": {"type": "number"}, "segments": {"type": "integer"}, "output_layer": {"type": "string"}, "db": {"type": "string"}}, "required": ["layer_id"]}},
            {"name": "processing.grid", "description": "Build a rect or hex grid over a bbox: south/west/north/east required; cell_size meters (default 500), hex flag, output_layer, db optional; persists to db", "inputSchema": {"type": "object", "properties": {"south": {"type": "number"}, "west": {"type": "number"}, "north": {"type": "number"}, "east": {"type": "number"}, "cell_size": {"type": "number"}, "hex": {"type": "boolean"}, "output_layer": {"type": "string"}, "db": {"type": "string"}}, "required": ["south", "west", "north", "east"]}},
            {"name": "processing.nearest", "description": "Nearest-neighbor join: hub lines from source_layer to closest target_layer feature; output_layer, db optional; persists to db", "inputSchema": {"type": "object", "properties": {"source_layer": {"type": "string"}, "target_layer": {"type": "string"}, "output_layer": {"type": "string"}, "db": {"type": "string"}}, "required": ["source_layer", "target_layer"]}},
            {"name": "processing.geodesic", "description": "True-distance shape: circle (radius_m), ellipse (major_m/minor_m/bearing_deg) or line (lng2/lat2) around lng/lat; segments, output_layer, db optional; persists to db", "inputSchema": {"type": "object", "properties": {"shape": {"type": "string"}, "lng": {"type": "number"}, "lat": {"type": "number"}, "radius_m": {"type": "number"}, "major_m": {"type": "number"}, "minor_m": {"type": "number"}, "bearing_deg": {"type": "number"}, "lng2": {"type": "number"}, "lat2": {"type": "number"}, "segments": {"type": "integer"}, "output_layer": {"type": "string"}, "db": {"type": "string"}}, "required": ["shape", "lng", "lat"]}},
            {"name": "map.forge", "description": "Forge a map from a MapSpec JSON file: path required; live=false dry-runs QL per layer else executes; ingest=true writes live hits to db", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "live": {"type": "boolean"}, "ingest": {"type": "boolean"}, "db": {"type": "string"}}, "required": ["path"]}},
            {"name": "wkt.show", "description": "Show WKT geometry as a layer: wkt required (POINT/LINESTRING/POLYGON + MULTI, 2D only); layer_id, db optional; persists to db", "inputSchema": {"type": "object", "properties": {"wkt": {"type": "string"}, "layer_id": {"type": "string"}, "db": {"type": "string"}}, "required": ["wkt"]}},
            {"name": "doppler.locate", "description": "Doppler geolocation: rank grid candidates by matching curves to ADS-B tracks; path to capture JSON required; bbox, step_deg, top_n, terrain_m, output_layer, db optional; persists top candidates to db", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "south": {"type": "number"}, "west": {"type": "number"}, "north": {"type": "number"}, "east": {"type": "number"}, "step_deg": {"type": "number"}, "top_n": {"type": "integer"}, "terrain_m": {"type": "number"}, "output_layer": {"type": "string"}, "db": {"type": "string"}}, "required": ["path"]}},
            {"name": "export.png", "description": "Export map PNG (Phase 1 render path)", "inputSchema": {"type": "object", "properties": {"width": {"type": "integer"}, "height": {"type": "integer"}}}},
        ]
    })
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn call_tool(name: &str, args: &Value, connect: Option<u16>) -> Value {
    // Connected mode: the DB-touching tools run in the LIVE app instead of
    // the local headless path, so there is exactly one writer. Everything
    // else (network tools, feature reads, export) stays local.
    if let Some(port) = connect {
        let forward = matches!(
            name,
            "project.status"
                | "layer.add"
                | "layer.list"
                | "layer.remove"
                | "layer.color"
                | "layer.import"
                | "processing.buffer"
        );
        if forward {
            let result = forward_control(port, name, args.clone());
            return json!({"content": [{"type": "text", "text": result.to_string()}]});
        }
    }
    let result = match name {
        "status" => run(Command::Status),
        "project.status" => run(Command::ProjectStatus),
        "layer.add" => {
            let db = str_arg(args, "db");
            layer_add(
                db.as_deref(),
                str_arg(args, "id").unwrap_or_default().as_str(),
                str_arg(args, "name").unwrap_or_default().as_str(),
                str_arg(args, "source"),
                str_arg(args, "geom_type"),
                str_arg(args, "crs"),
                args.get("visible").and_then(|v| v.as_bool()),
            )
        }
        "layer.list" => {
            let db = str_arg(args, "db");
            layer_list(db.as_deref())
        }
        "layer.remove" => {
            let db = str_arg(args, "db");
            layer_remove(
                db.as_deref(),
                str_arg(args, "id").unwrap_or_default().as_str(),
            )
        }
        "layer.color" => {
            let db = str_arg(args, "db");
            layer_color_set(
                db.as_deref(),
                str_arg(args, "id").unwrap_or_default().as_str(),
                str_arg(args, "color").unwrap_or_default().as_str(),
            )
        }
        "geocode.search" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).and_then(|n| {
                if (1..=50).contains(&n) {
                    u8::try_from(n).ok()
                } else {
                    None
                }
            });
            // Out-of-range limit is a typed error, not a silent clamp.
            if args.get("limit").is_some() && limit.is_none() {
                json!({"error": "invalid input: limit must be 1..50", "op": "geocode.search"})
            } else {
                run(Command::GeocodeSearch {
                    query: str_arg(args, "query").unwrap_or_default(),
                    limit,
                    live: args.get("live").and_then(|v| v.as_bool()).unwrap_or(false),
                })
            }
        }
        "overpass.query" => {
            let f = |k: &str| args.get(k).and_then(|v| v.as_f64());
            let live = args.get("live").and_then(|v| v.as_bool()).unwrap_or(false);
            let ingest = args
                .get("ingest")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let db = str_arg(args, "db");
            if ingest && !live {
                return json!({"content": [{"type": "text", "text": json!({"error": "invalid input: ingest requires live=true (nothing to ingest from a dry-run)", "op": "overpass.query"}).to_string()}]});
            }
            let out = run(Command::OverpassQuery {
                key: str_arg(args, "key").unwrap_or_default(),
                value: str_arg(args, "value").unwrap_or_default(),
                south: f("south"),
                west: f("west"),
                north: f("north"),
                east: f("east"),
                live,
            });
            if ingest && live && out.get("error").is_none() {
                let key = out
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("overpass");
                let value = out
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("result");
                let slug = format!("{key}-{value}")
                    .chars()
                    .map(|c| if c.is_whitespace() { '_' } else { c })
                    .collect::<String>();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let stored = ingest_hits(
                    db.as_deref(),
                    &slug,
                    format!("{key}={value}").as_str(),
                    "overpass",
                    "point",
                    &hits,
                    "overpass.query",
                );
                return json!({"content": [{"type": "text", "text": json!({"query_result": out, "ingest": stored}).to_string()}]});
            }
            out
        }
        "features.list" => {
            let db = str_arg(args, "db");
            features_list(db.as_deref(), str_arg(args, "layer_id").as_deref())
        }
        "layer.import" => {
            let out = run(Command::LayerImport {
                path: str_arg(args, "path").unwrap_or_default(),
                format: str_arg(args, "format"),
                layer_id: str_arg(args, "layer_id"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let layer = out
                .get("layer")
                .and_then(|v| v.as_str())
                .unwrap_or("import");
            let name = out.get("name").and_then(|v| v.as_str()).unwrap_or("import");
            let hits = out.get("hits").cloned().unwrap_or(Value::Null);
            let geom = infer_geom_type(&hits);
            let stored = ingest_hits(
                db.as_deref(),
                layer,
                name,
                "import",
                geom,
                &hits,
                "layer.import",
            );
            json!({"import_result": out, "ingest": stored})
        }
        "wkt.show" => {
            let out = run(Command::WktShow {
                wkt: str_arg(args, "wkt").unwrap_or_default(),
                layer_id: str_arg(args, "layer_id"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let layer = out.get("layer").and_then(|v| v.as_str()).unwrap_or("wkt");
            let name = out.get("name").and_then(|v| v.as_str()).unwrap_or("wkt");
            let hits = out.get("hits").cloned().unwrap_or(Value::Null);
            let geom = infer_geom_type(&hits);
            let stored = ingest_hits(db.as_deref(), layer, name, "wkt", geom, &hits, "wkt.show");
            json!({"wkt_result": out, "ingest": stored})
        }
        "doppler.locate" => {
            let out = run(Command::DopplerLocate {
                path: str_arg(args, "path").unwrap_or_default(),
                south: args.get("south").and_then(|v| v.as_f64()),
                west: args.get("west").and_then(|v| v.as_f64()),
                north: args.get("north").and_then(|v| v.as_f64()),
                east: args.get("east").and_then(|v| v.as_f64()),
                step_deg: args.get("step_deg").and_then(|v| v.as_f64()),
                top_n: args
                    .get("top_n")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| usize::try_from(n).ok()),
                terrain_m: args.get("terrain_m").and_then(|v| v.as_f64()),
                output_layer: str_arg(args, "output_layer"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let layer = out
                .get("layer")
                .and_then(|v| v.as_str())
                .unwrap_or("doppler-hits");
            let name = out
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("doppler-hits");
            let hits = out.get("hits").cloned().unwrap_or(Value::Null);
            let stored = ingest_hits(
                db.as_deref(),
                layer,
                name,
                "doppler",
                "point",
                &hits,
                "doppler.locate",
            );
            json!({"doppler_result": out, "ingest": stored})
        }
        "processing.buffer" => {
            let layer_id = str_arg(args, "layer_id").unwrap_or_default();
            let distance = args
                .get("distance")
                .or_else(|| args.get("distance_m"))
                .and_then(|v| v.as_f64())
                .unwrap_or(200.0);
            let segments = args
                .get("segments")
                .and_then(|v| v.as_u64())
                .and_then(|n| usize::try_from(n).ok());
            let output_layer = str_arg(args, "output_layer");
            let db = str_arg(args, "db");
            layer_buffer(
                db.as_deref(),
                &layer_id,
                distance,
                segments,
                output_layer.as_deref(),
            )
        }
        "processing.grid" => {
            let out = run(Command::ProcessingGrid {
                south: args.get("south").and_then(|v| v.as_f64()),
                west: args.get("west").and_then(|v| v.as_f64()),
                north: args.get("north").and_then(|v| v.as_f64()),
                east: args.get("east").and_then(|v| v.as_f64()),
                cell_size_m: args
                    .get("cell_size")
                    .or_else(|| args.get("cell_size_m"))
                    .and_then(|v| v.as_f64()),
                hex: args.get("hex").and_then(|v| v.as_bool()).unwrap_or(false),
                output_layer: str_arg(args, "output_layer"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let layer = out.get("layer").and_then(|v| v.as_str()).unwrap_or("grid");
            let name = out.get("name").and_then(|v| v.as_str()).unwrap_or("grid");
            let hits = out.get("hits").cloned().unwrap_or(Value::Null);
            let stored = ingest_hits(
                db.as_deref(),
                layer,
                name,
                "processing:grid",
                "polygon",
                &hits,
                "processing.grid",
            );
            json!({"grid_result": out, "ingest": stored})
        }
        "processing.nearest" => {
            let out = run(Command::ProcessingNearest {
                source_layer: str_arg(args, "source_layer").unwrap_or_default(),
                target_layer: str_arg(args, "target_layer").unwrap_or_default(),
                output_layer: str_arg(args, "output_layer"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let source = out
                .get("source_layer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let target = out
                .get("target_layer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let out_id = out
                .get("output_layer")
                .and_then(|v| v.as_str())
                .unwrap_or("nearest")
                .to_string();
            let conn = match store::open_at(db.as_deref()) {
                Ok(c) => c,
                Err(e) => return json!({"error": e.to_string(), "op": "processing.nearest"}),
            };
            let load = |id: &str| -> Result<Vec<qgiscraft_core::import::RawFeature>, String> {
                match store::list_features(&conn, Some(id)) {
                    Ok(rows) => Ok(rows
                        .into_iter()
                        .map(|f| qgiscraft_core::import::RawFeature {
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
            let src_feats = match load(&source) {
                Ok(f) => f,
                Err(e) => return json!({"error": e, "op": "processing.nearest"}),
            };
            let tgt_feats = match load(&target) {
                Ok(f) => f,
                Err(e) => return json!({"error": e, "op": "processing.nearest"}),
            };
            let lines = match qgiscraft_core::processing::nearest_join(&src_feats, &tgt_feats) {
                Ok(l) => l,
                Err(e) => return json!({"error": e.to_string(), "op": "processing.nearest"}),
            };
            let count = lines.len();
            let hits_vec: Vec<Value> = lines
                .iter()
                .map(|f| {
                    let geom: Value = serde_json::from_str(
                        f.geom_json.as_deref().unwrap_or("null"),
                    )
                    .unwrap_or(Value::Null);
                    json!({"name": f.name, "kind": f.kind, "lng": f.lng, "lat": f.lat, "geom": geom})
                })
                .collect();
            let hits = Value::Array(hits_vec);
            let stored = ingest_hits(
                db.as_deref(),
                &out_id,
                &out_id,
                "processing:nearest",
                "line",
                &hits,
                "processing.nearest",
            );
            json!({"nearest_result": out, "layer": out_id, "count": count, "ingest": stored})
        }
        "processing.geodesic" => {
            let out = run(Command::ProcessingGeodesic {
                shape: str_arg(args, "shape").unwrap_or_default(),
                lng: args.get("lng").and_then(|v| v.as_f64()),
                lat: args.get("lat").and_then(|v| v.as_f64()),
                radius_m: args.get("radius_m").and_then(|v| v.as_f64()),
                major_m: args.get("major_m").and_then(|v| v.as_f64()),
                minor_m: args.get("minor_m").and_then(|v| v.as_f64()),
                bearing_deg: args.get("bearing_deg").and_then(|v| v.as_f64()),
                lng2: args.get("lng2").and_then(|v| v.as_f64()),
                lat2: args.get("lat2").and_then(|v| v.as_f64()),
                segments: args
                    .get("segments")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| usize::try_from(n).ok()),
                output_layer: str_arg(args, "output_layer"),
            });
            if out.get("error").is_some() {
                return json!({"content": [{"type": "text", "text": out.to_string()}]});
            }
            let db = str_arg(args, "db");
            let layer = out
                .get("layer")
                .and_then(|v| v.as_str())
                .unwrap_or("geodesic");
            let name = out
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("geodesic");
            let hits = out.get("hits").cloned().unwrap_or(Value::Null);
            let geom = infer_geom_type(&hits);
            let stored = ingest_hits(
                db.as_deref(),
                layer,
                name,
                "processing:geodesic",
                geom,
                &hits,
                "processing.geodesic",
            );
            json!({"geodesic_result": out, "ingest": stored})
        }
        "map.forge" => {
            let live = args.get("live").and_then(|v| v.as_bool()).unwrap_or(false);
            let ingest = args
                .get("ingest")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let db = str_arg(args, "db");
            if ingest && !live {
                return json!({"content": [{"type": "text", "text": json!({"error": "invalid input: ingest requires live=true (nothing to ingest from a dry-run)", "op": "map.forge"}).to_string()}]});
            }
            let out = run(Command::MapForge {
                path: str_arg(args, "path").unwrap_or_default(),
                live,
            });
            if out.get("error").is_some() {
                return out;
            }
            if ingest && live {
                let slug = out
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or("mapforge")
                    .to_string();
                let mut ingested: Vec<Value> = Vec::new();
                if let Some(layers) = out.get("layers").and_then(|v| v.as_array()) {
                    for layer in layers {
                        let id = layer.get("id").and_then(|v| v.as_str()).unwrap_or("layer");
                        let name = layer.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                        let hits = layer.get("hits").cloned().unwrap_or(Value::Null);
                        ingested.push(ingest_hits(
                            db.as_deref(),
                            id,
                            name,
                            format!("mapforge:{slug}").as_str(),
                            "point",
                            &hits,
                            "map.forge",
                        ));
                    }
                }
                return json!({"content": [{"type": "text", "text": json!({"forge_result": out, "ingest": ingested}).to_string()}]});
            }
            out
        }
        "export.png" => run(Command::ExportPng {
            width: args
                .get("width")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok()),
            height: args
                .get("height")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok()),
        }),
        _ => json!({"error": format!("unknown tool: {name}")}),
    };
    json!({"content": [{"type": "text", "text": result.to_string()}]})
}

/// Minimal MCP 2024-11-05 server over stdio. Line-delimited JSON-RPC.
/// With `connect`, the DB-touching tools run in the live app instead.
fn run_mcp_stdio(connect: Option<u16>) {
    if let Some(port) = connect {
        eprintln!("qgiscraft-mcp: MCP server on stdio, connected to app 127.0.0.1:{port}");
    } else {
        eprintln!("qgiscraft-mcp: MCP server listening on stdio");
    }
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(e) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {"code": -32700, "message": format!("parse error: {e}")}
                });
                if let Ok(text) = serde_json::to_string(&resp) {
                    let _ = writeln!(out, "{text}");
                    let _ = out.flush();
                }
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let is_notification = req.get("id").is_none();

        let result: Option<Value> = match method {
            "initialize" => Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "qgiscraft-mcp", "version": env!("CARGO_PKG_VERSION")},
            })),
            "tools/list" => Some(tools_list()),
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(json!({}));
                let tname = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let targs = params.get("arguments").cloned().unwrap_or(json!({}));
                Some(call_tool(tname, &targs, connect))
            }
            "ping" => Some(json!({})),
            _ => {
                if is_notification {
                    None
                } else {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32601, "message": format!("unknown method: {method}")}
                    });
                    if let Ok(text) = serde_json::to_string(&resp) {
                        let _ = writeln!(out, "{text}");
                        let _ = out.flush();
                    }
                    continue;
                }
            }
        };

        if is_notification {
            continue;
        }
        let body = result.unwrap_or(json!({}));
        let resp = json!({"jsonrpc": "2.0", "id": id, "result": body});
        if let Ok(text) = serde_json::to_string(&resp) {
            let _ = writeln!(out, "{text}");
            let _ = out.flush();
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Status { json } => show(run(Command::Status), json, "qgiscraft-mcp status"),
        Commands::ProjectStatus { json } => show(run(Command::ProjectStatus), json, "project"),
        Commands::LayerAdd {
            id,
            name,
            source,
            geom_type,
            crs,
            visible,
            db,
            json,
        } => show(
            layer_add(
                db.as_deref(),
                &id,
                &name,
                Some(source),
                Some(geom_type),
                crs,
                Some(visible),
            ),
            json,
            "layer.add",
        ),
        Commands::LayerList { db, json } => {
            show(layer_list(db.as_deref()), json, "layers");
        }
        Commands::LayerRemove { id, db, json } => {
            show(layer_remove(db.as_deref(), &id), json, "layer.remove");
        }
        Commands::LayerColor {
            id,
            color,
            db,
            json,
        } => {
            show(
                layer_color_set(db.as_deref(), &id, &color),
                json,
                "layer.color",
            );
        }
        Commands::GeocodeSearch {
            query,
            limit,
            live,
            json,
        } => show(
            run(Command::GeocodeSearch { query, limit, live }),
            json,
            "geocode.search",
        ),
        Commands::OverpassQuery {
            key,
            value,
            south,
            west,
            north,
            east,
            live,
            ingest,
            db,
            json,
        } => {
            if ingest && !live {
                show(
                    json!({"error": "invalid input: ingest requires --live (nothing to ingest from a dry-run)", "op": "overpass.query"}),
                    json,
                    "overpass.query",
                );
            } else {
                let out = run(Command::OverpassQuery {
                    key: key.clone(),
                    value: value.clone(),
                    south,
                    west,
                    north,
                    east,
                    live,
                });
                if ingest && live && out.get("error").is_none() {
                    let slug = format!("{key}-{value}")
                        .chars()
                        .map(|c| if c.is_whitespace() { '_' } else { c })
                        .collect::<String>();
                    let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                    let stored = ingest_hits(
                        db.as_deref(),
                        &slug,
                        format!("{key}={value}").as_str(),
                        "overpass",
                        "point",
                        &hits,
                        "overpass.query",
                    );
                    show(
                        json!({"query_result": out, "ingest": stored}),
                        json,
                        "overpass.query",
                    );
                } else {
                    show(out, json, "overpass.query");
                }
            }
        }
        Commands::FeaturesList { layer_id, db, json } => show(
            features_list(db.as_deref(), layer_id.as_deref()),
            json,
            "features.list",
        ),
        Commands::LayerImport {
            path,
            format,
            layer_id,
            db,
            json,
        } => {
            let out = run(Command::LayerImport {
                path,
                format,
                layer_id,
            });
            if out.get("error").is_some() {
                show(out, json, "layer.import");
            } else {
                let layer = out
                    .get("layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("import")
                    .to_string();
                let name = out
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("import")
                    .to_string();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let geom = infer_geom_type(&hits);
                let stored = ingest_hits(
                    db.as_deref(),
                    &layer,
                    &name,
                    "import",
                    geom,
                    &hits,
                    "layer.import",
                );
                show(
                    json!({"import_result": out, "ingest": stored}),
                    json,
                    "layer.import",
                );
            }
        }
        Commands::ProcessingBuffer {
            layer_id,
            distance,
            segments,
            output_layer,
            db,
            json,
        } => {
            let result = layer_buffer(
                db.as_deref(),
                &layer_id,
                distance,
                segments,
                output_layer.as_deref(),
            );
            show(result, json, "processing.buffer");
        }
        Commands::ProcessingGrid {
            south,
            west,
            north,
            east,
            cell_size,
            hex,
            output_layer,
            db,
            json,
        } => {
            let out = run(Command::ProcessingGrid {
                south: Some(south),
                west: Some(west),
                north: Some(north),
                east: Some(east),
                cell_size_m: Some(cell_size),
                hex,
                output_layer,
            });
            if out.get("error").is_some() {
                show(out, json, "processing.grid");
            } else {
                let layer = out
                    .get("layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("grid")
                    .to_string();
                let name = out
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("grid")
                    .to_string();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let stored = ingest_hits(
                    db.as_deref(),
                    &layer,
                    &name,
                    "processing:grid",
                    "polygon",
                    &hits,
                    "processing.grid",
                );
                show(
                    json!({"grid_result": out, "ingest": stored}),
                    json,
                    "processing.grid",
                );
            }
        }
        Commands::ProcessingNearest {
            source_layer,
            target_layer,
            output_layer,
            db,
            json,
        } => {
            let out = run(Command::ProcessingNearest {
                source_layer: source_layer.clone(),
                target_layer: target_layer.clone(),
                output_layer: output_layer.clone(),
            });
            if out.get("error").is_some() {
                show(out, json, "processing.nearest");
            } else {
                let out_id = out
                    .get("output_layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("nearest")
                    .to_string();
                let conn = match store::open_at(db.as_deref()) {
                    Ok(c) => c,
                    Err(e) => {
                        show(
                            json!({"error": e.to_string(), "op": "processing.nearest"}),
                            json,
                            "processing.nearest",
                        );
                        return;
                    }
                };
                let load = |id: &str| -> Result<Vec<qgiscraft_core::import::RawFeature>, String> {
                    match store::list_features(&conn, Some(id)) {
                        Ok(rows) => Ok(rows
                            .into_iter()
                            .map(|f| qgiscraft_core::import::RawFeature {
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
                let joined = load(&source_layer)
                    .and_then(|s| load(&target_layer).map(|t| (s, t)))
                    .and_then(|(s, t)| {
                        qgiscraft_core::processing::nearest_join(&s, &t).map_err(|e| e.to_string())
                    });
                match joined {
                    Err(e) => show(
                        json!({"error": e, "op": "processing.nearest"}),
                        json,
                        "processing.nearest",
                    ),
                    Ok(lines) => {
                        let count = lines.len();
                        let hits_vec: Vec<Value> = lines
                            .iter()
                            .map(|f| {
                                let geom: Value = serde_json::from_str(
                                    f.geom_json.as_deref().unwrap_or("null"),
                                )
                                .unwrap_or(Value::Null);
                                json!({"name": f.name, "kind": f.kind, "lng": f.lng, "lat": f.lat, "geom": geom})
                            })
                            .collect();
                        let hits = Value::Array(hits_vec);
                        let stored = ingest_hits(
                            db.as_deref(),
                            &out_id,
                            &out_id,
                            "processing:nearest",
                            "line",
                            &hits,
                            "processing.nearest",
                        );
                        show(
                            json!({"nearest_result": out, "layer": out_id, "count": count, "ingest": stored}),
                            json,
                            "processing.nearest",
                        );
                    }
                }
            }
        }
        Commands::ProcessingGeodesic {
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
            db,
            json,
        } => {
            let out = run(Command::ProcessingGeodesic {
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
            });
            if out.get("error").is_some() {
                show(out, json, "processing.geodesic");
            } else {
                let layer = out
                    .get("layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("geodesic")
                    .to_string();
                let name = out
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("geodesic")
                    .to_string();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let geom = infer_geom_type(&hits);
                let stored = ingest_hits(
                    db.as_deref(),
                    &layer,
                    &name,
                    "processing:geodesic",
                    geom,
                    &hits,
                    "processing.geodesic",
                );
                show(
                    json!({"geodesic_result": out, "ingest": stored}),
                    json,
                    "processing.geodesic",
                );
            }
        }
        Commands::ForgeFromSpec {
            path,
            live,
            ingest,
            db,
            json,
        } => {
            if ingest && !live {
                show(
                    json!({"error": "invalid input: ingest requires --live (nothing to ingest from a dry-run)", "op": "map.forge"}),
                    json,
                    "map.forge",
                );
            } else {
                let out = run(Command::MapForge { path, live });
                if out.get("error").is_some() || !(ingest && live) {
                    show(out, json, "map.forge");
                } else {
                    let slug = out
                        .get("slug")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mapforge")
                        .to_string();
                    let mut ingested: Vec<Value> = Vec::new();
                    if let Some(layers) = out.get("layers").and_then(|v| v.as_array()) {
                        for layer in layers {
                            let id = layer.get("id").and_then(|v| v.as_str()).unwrap_or("layer");
                            let name = layer.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                            let hits = layer.get("hits").cloned().unwrap_or(Value::Null);
                            ingested.push(ingest_hits(
                                db.as_deref(),
                                id,
                                name,
                                format!("mapforge:{slug}").as_str(),
                                "point",
                                &hits,
                                "map.forge",
                            ));
                        }
                    }
                    show(
                        json!({"forge_result": out, "ingest": ingested}),
                        json,
                        "map.forge",
                    );
                }
            }
        }
        Commands::WktShow {
            wkt,
            layer_id,
            db,
            json,
        } => {
            let out = run(Command::WktShow { wkt, layer_id });
            if out.get("error").is_some() {
                show(out, json, "wkt.show");
            } else {
                let layer = out
                    .get("layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("wkt")
                    .to_string();
                let name = out
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("wkt")
                    .to_string();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let geom = infer_geom_type(&hits);
                let stored =
                    ingest_hits(db.as_deref(), &layer, &name, "wkt", geom, &hits, "wkt.show");
                show(
                    json!({"wkt_result": out, "ingest": stored}),
                    json,
                    "wkt.show",
                );
            }
        }
        Commands::DopplerLocate {
            path,
            south,
            west,
            north,
            east,
            step_deg,
            top_n,
            terrain_m,
            output_layer,
            db,
            json,
        } => {
            let out = run(Command::DopplerLocate {
                path,
                south,
                west,
                north,
                east,
                step_deg,
                top_n,
                terrain_m,
                output_layer,
            });
            if out.get("error").is_some() {
                show(out, json, "doppler.locate");
            } else {
                let layer = out
                    .get("layer")
                    .and_then(|v| v.as_str())
                    .unwrap_or("doppler-hits")
                    .to_string();
                let name = out
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("doppler-hits")
                    .to_string();
                let hits = out.get("hits").cloned().unwrap_or(Value::Null);
                let stored = ingest_hits(
                    db.as_deref(),
                    &layer,
                    &name,
                    "doppler",
                    "point",
                    &hits,
                    "doppler.locate",
                );
                show(
                    json!({"doppler_result": out, "ingest": stored}),
                    json,
                    "doppler.locate",
                );
            }
        }
        Commands::Mcp { connect } => run_mcp_stdio(connect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_eleven_tools() {
        let catalog = tools_list();
        let tools = catalog
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(tools.len(), 18);
    }

    #[test]
    fn unknown_tool_becomes_error_content_not_panic() {
        let result = call_tool("nope.missing", &json!({}), None);
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("unknown tool"));
    }

    #[test]
    fn layer_add_empty_id_is_typed_error() {
        let result = call_tool("layer.add", &json!({"id": "", "name": "x"}), None);
        assert!(content_text(&result).contains("invalid input"));
    }

    #[test]
    fn map_forge_dry_run_returns_single_envelope() {
        let mut p = std::env::temp_dir();
        p.push(format!("qgiscraft-test-forge-{}.json", std::process::id()));
        let doc = r#"{"title":"T","slug":"t-spot","bbox":{"south":48.19,"west":16.35,"north":48.23,"east":16.40},"layers":[{"id":"pharm","key":"amenity","value":"pharmacy"}]}"#;
        if std::fs::write(&p, doc).is_err() {
            return;
        }
        let result = call_tool(
            "map.forge",
            &json!({"path": p.to_string_lossy().to_string()}),
            None,
        );
        let _ = std::fs::remove_file(&p);
        let text = content_text(&result);
        assert!(text.contains("t-spot"));
        assert!(text.contains("out geom 100"));
    }

    #[test]
    fn map_forge_ingest_without_live_is_typed_error() {
        let result = call_tool(
            "map.forge",
            &json!({"path": "x.json", "live": false, "ingest": true}),
            None,
        );
        assert!(content_text(&result).contains("ingest requires live"));
    }

    #[test]
    fn wkt_show_persists_point_to_temp_db() {
        let db = temp_db("wkt");
        let result = call_tool(
            "wkt.show",
            &json!({"wkt": "POINT (16.3738 48.2082)", "db": db}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("wkt-1"));
        let _ = std::fs::remove_file(temp_db("wkt"));
    }

    #[test]
    fn wkt_show_bad_input_is_typed_error() {
        let result = call_tool("wkt.show", &json!({"wkt": "POINT (16.3)"}), None);
        assert!(content_text(&result).contains("invalid input"));
    }

    #[test]
    fn doppler_locate_persists_candidates_to_temp_db() {
        let mut cap_path = std::env::temp_dir();
        cap_path.push(format!("qgiscraft-test-cap-{}.json", std::process::id()));
        let mut doc = String::from("{\"tracks\": [[");
        for i in 0..30 {
            if i > 0 {
                doc.push(',');
            }
            doc.push_str(&format!(
                "{{\"t\":{i},\"lng\":{},\"lat\":48.2,\"alt_m\":500.0,\"speed_ms\":60.0,\"heading_deg\":90.0}}",
                16.30 + i as f64 * 0.001
            ));
        }
        doc.push_str("]], \"curves\": [{\"f0\": 120.0, \"samples\": [[5.0,125.0],[10.0,122.0],[15.0,118.0],[20.0,114.0],[25.0,110.0]]}]}");
        let _ = std::fs::write(&cap_path, &doc);
        let db = temp_db("doppler");
        let result = call_tool(
            "doppler.locate",
            &json!({"path": cap_path.to_string_lossy().to_string(), "south": 48.19, "west": 16.31, "north": 48.23, "east": 16.35, "step_deg": 0.01, "top_n": 5, "db": db}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("doppler-hits"));
        assert!(text.contains("\"count\":5"));
        let _ = std::fs::remove_file(&cap_path);
        let _ = std::fs::remove_file(temp_db("doppler"));
    }

    #[test]
    fn doppler_locate_missing_file_is_typed_error() {
        let result = call_tool(
            "doppler.locate",
            &json!({"path": "definitely-not-here-doppler-xyz.json", "south": 48.19, "west": 16.31, "north": 48.23, "east": 16.35}),
            None,
        );
        assert!(content_text(&result).contains("read failed"));
    }

    #[test]
    fn processing_geodesic_circle_persists_to_temp_db() {
        let db = temp_db("geodesic");
        let result = call_tool(
            "processing.geodesic",
            &json!({"shape": "circle", "lng": 16.3738, "lat": 48.2082, "radius_m": 1000.0, "db": db}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("geodesic-circle"));
        assert!(text.contains("\"count\":1"));
        let _ = std::fs::remove_file(temp_db("geodesic"));
    }

    #[test]
    fn processing_geodesic_bad_shape_is_typed_error() {
        let result = call_tool(
            "processing.geodesic",
            &json!({"shape": "square", "lng": 16.3, "lat": 48.2, "radius_m": 100.0}),
            None,
        );
        assert!(content_text(&result).contains("circle, ellipse or line"));
    }

    #[test]
    fn layer_color_roundtrip_in_temp_db() {
        let db = temp_db("color");
        let _ = call_tool(
            "layer.add",
            &json!({"id": "c1", "name": "C1", "db": db}),
            None,
        );
        let set = call_tool(
            "layer.color",
            &json!({"id": "c1", "color": "#ff0000", "db": db}),
            None,
        );
        assert!(content_text(&set).contains("#ff0000"));
        let listed = call_tool("layer.list", &json!({"db": db}), None);
        assert!(content_text(&listed).contains("#ff0000"));
        let cleared = call_tool(
            "layer.color",
            &json!({"id": "c1", "color": "", "db": db}),
            None,
        );
        assert!(content_text(&cleared).contains("\"updated\":true"));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn layer_color_bad_hex_is_typed_error() {
        let result = call_tool("layer.color", &json!({"id": "c1", "color": "red"}), None);
        assert!(content_text(&result).contains("invalid input"));
    }

    #[test]
    fn processing_grid_persists_cells_to_temp_db() {
        let db = temp_db("grid");
        let result = call_tool(
            "processing.grid",
            &json!({"south": 48.20, "west": 16.36, "north": 48.21, "east": 16.37, "cell_size": 500.0, "db": db}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("cell-0-0"));
        let _ = std::fs::remove_file(temp_db("grid"));
    }

    #[test]
    fn processing_grid_missing_bbox_is_typed_error() {
        let result = call_tool("processing.grid", &json!({}), None);
        assert!(content_text(&result).contains("south/west/north/east"));
    }

    #[test]
    fn processing_nearest_joins_layers_in_temp_db() {
        let db = temp_db("nearest");
        let src = json!([{"name": "bike-1", "kind": "point", "lng": 16.37, "lat": 48.20}]);
        let tgt = json!([
            {"name": "near-pharm", "kind": "point", "lng": 16.371, "lat": 48.2005},
            {"name": "far-pharm", "kind": "point", "lng": 16.40, "lat": 48.22},
        ]);
        let _ = ingest_hits(
            Some(db.as_str()),
            "bikes",
            "bikes",
            "test",
            "point",
            &src,
            "test",
        );
        let _ = ingest_hits(
            Some(db.as_str()),
            "pharms",
            "pharms",
            "test",
            "point",
            &tgt,
            "test",
        );
        let result = call_tool(
            "processing.nearest",
            &json!({"source_layer": "bikes", "target_layer": "pharms", "db": db}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("bikes-nearest"));
        assert!(text.contains("\"count\":1"));
        let _ = std::fs::remove_file(temp_db("nearest"));
    }

    #[test]
    fn processing_nearest_empty_source_is_typed_error() {
        let result = call_tool(
            "processing.nearest",
            &json!({"source_layer": "", "target_layer": "x"}),
            None,
        );
        assert!(content_text(&result).contains("invalid input"));
    }

    fn content_text(result: &Value) -> String {
        result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn temp_db(tag: &str) -> String {
        let mut p = std::env::temp_dir();
        p.push(format!("qgiscraft-test-{}-{tag}.db", std::process::id()));
        let s = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&p);
        s
    }

    #[test]
    fn layer_persist_roundtrip_in_temp_db() {
        let db = temp_db("persist");
        let added = layer_add(
            Some(db.as_str()),
            "t1",
            "Test One",
            Some("osm".to_string()),
            Some("point".to_string()),
            None,
            Some(true),
        );
        assert!(added
            .get("added")
            .and_then(|v| v.as_bool())
            .unwrap_or(false));
        let listed = layer_list(Some(db.as_str()));
        assert_eq!(listed.get("count").and_then(|v| v.as_u64()), Some(1));
        let removed = layer_remove(Some(db.as_str()), "t1");
        assert_eq!(removed.get("removed").and_then(|v| v.as_bool()), Some(true));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn overpass_dry_run_returns_ql_offline() {
        let result = call_tool(
            "overpass.query",
            &json!({"key": "amenity", "value": "pharmacy"}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("amenity"));
        assert!(text.contains("\"live\":false"));
    }

    #[test]
    fn overpass_hostile_key_rejected_without_network() {
        let result = call_tool(
            "overpass.query",
            &json!({"key": "amenity\";", "value": "x"}),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("invalid input"));
    }

    #[test]
    fn overpass_ingest_requires_live() {
        let result = call_tool(
            "overpass.query",
            &json!({"key": "amenity", "value": "pharmacy", "ingest": true}),
            None,
        );
        assert!(content_text(&result).contains("ingest requires live=true"));
    }

    #[test]
    fn ingest_hits_roundtrip_in_temp_db() {
        let db = temp_db("ingest");
        let hits = json!([
            {"name": "A", "kind": "node", "lng": 16.3, "lat": 48.2},
            {"name": "Broken", "kind": "node", "lng": "east", "lat": 48.2},
        ]);
        let stored = ingest_hits(
            Some(db.as_str()),
            "amenity-pharmacy",
            "amenity=pharmacy",
            "overpass",
            "point",
            &hits,
            "overpass.query",
        );
        assert_eq!(stored.get("ingested").and_then(|v| v.as_u64()), Some(1));
        let listed = features_list(Some(db.as_str()), Some("amenity-pharmacy"));
        assert_eq!(listed.get("count").and_then(|v| v.as_u64()), Some(1));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn connected_mode_forwards_to_live_app() {
        use std::io::{BufRead, BufReader, Write};
        let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(_) => return,
        };
        let port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(_) => return,
        };
        std::thread::spawn(move || {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(_) => return,
            };
            let mut reader = match stream.try_clone() {
                Ok(s) => BufReader::new(s),
                Err(_) => return,
            };
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let req: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
            let id = req.get("id").cloned().unwrap_or(Value::Null);
            let resp = serde_json::json!({"id": id, "ok": true, "result": {"count": 3}});
            if let Ok(text) = serde_json::to_string(&resp) {
                let _ = writeln!(stream, "{text}");
                let _ = stream.flush();
            }
        });
        // With connect=Some the result must come from the stub app even
        // though the local DB path does not exist.
        let result = call_tool(
            "layer.list",
            &serde_json::json!({"db": "no-such-db-xyz.db"}),
            Some(port),
        );
        let text = content_text(&result);
        assert!(text.contains("\"count\":3"), "got: {text}");
    }

    #[test]
    fn connected_mode_unreachable_is_typed_error() {
        let port = match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(l) => match l.local_addr() {
                Ok(a) => a.port(),
                Err(_) => return,
            },
            Err(_) => return,
        };
        // Listener dropped: nothing listens on this port anymore.
        let result = call_tool("layer.list", &serde_json::json!({}), Some(port));
        let text = content_text(&result);
        assert!(
            text.contains("control unreachable") || text.contains("error"),
            "got: {text}"
        );
    }

    #[test]
    fn layer_import_roundtrip_in_temp_db() {
        let dir = std::env::temp_dir();
        let mut src = dir.clone();
        src.push(format!("qgis-test-import-{}.geojson", std::process::id()));
        let doc = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"name":"A"},"geometry":{"type":"Point","coordinates":[16.3,48.2]}},{"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[16.3,48.2],[16.4,48.25]]}}]}"#;
        if std::fs::write(&src, doc).is_err() {
            return;
        }
        let db = temp_db("import");
        let result = call_tool(
            "layer.import",
            &serde_json::json!({"path": src.to_string_lossy(), "db": db}),
            None,
        );
        let text = content_text(&result);
        let _ = std::fs::remove_file(&src);
        assert!(text.contains("\"ingested\":2"), "got: {text}");
        let _ = std::fs::remove_file(temp_db("import"));
    }

    #[test]
    fn layer_import_missing_file_is_typed_error() {
        let result = call_tool(
            "layer.import",
            &serde_json::json!({"path": "definitely-not-here-xyz.geojson"}),
            None,
        );
        assert!(content_text(&result).contains("read failed"));
    }

    #[test]
    fn geocode_out_of_range_limit_rejected() {
        let result = call_tool(
            "geocode.search",
            &json!({"query": "Wien", "limit": 99}),
            None,
        );
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("limit must be 1..50"));
    }

    #[test]
    fn processing_buffer_roundtrip_in_temp_db() {
        let db = temp_db("buffer");
        let hits = json!([
            {"name": "Point1", "kind": "node", "lng": 16.37, "lat": 48.21},
        ]);
        let _ = ingest_hits(
            Some(db.as_str()),
            "source-layer",
            "Source Layer",
            "test",
            "point",
            &hits,
            "test",
        );
        let result = call_tool(
            "processing.buffer",
            &json!({
                "layer_id": "source-layer",
                "distance": 150.0,
                "segments": 16,
                "output_layer": "buffered-out",
                "db": db.as_str(),
            }),
            None,
        );
        let text = content_text(&result);
        assert!(text.contains("\"buffered\":1"), "got: {text}");
        assert!(
            text.contains("\"output_layer\":\"buffered-out\""),
            "got: {text}"
        );

        let listed = features_list(Some(db.as_str()), Some("buffered-out"));
        assert_eq!(listed.get("count").and_then(|v| v.as_u64()), Some(1));
        let _ = std::fs::remove_file(&db);
    }
}
