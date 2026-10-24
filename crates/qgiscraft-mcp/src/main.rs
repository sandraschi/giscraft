//! Stdout is pure JSON-RPC. Every log line goes to stderr via `eprintln!`.
//! The MCP layer never touches geo state; it dispatches `Command` IDs
//! into `qgiscraft-core` and frames the resulting `serde_json::Value`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::{Parser, Subcommand};
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
    /// Run the MCP server over stdio (logs on stderr, JSON-RPC on stdout)
    Mcp,
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
            json!({"id": l.id, "name": l.name, "source": l.source, "geom_type": l.geom_type, "crs": l.crs, "visible": l.visible, "features": n})
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

/// Ingest dispatch-result hits into the project DB under `layer_id`.
/// Pure-DB seam: tested with temp DBs, no network.
fn ingest_hits(db: Option<&str>, layer_id: &str, layer_name: &str, hits: &Value) -> Value {
    if layer_id.trim().is_empty() {
        return json!({"error": "invalid input: layer id must not be empty", "op": "overpass.query"});
    }
    let conn = match store::open_at(db) {
        Ok(conn) => conn,
        Err(e) => return json!({"error": e.to_string(), "op": "overpass.query"}),
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
            });
        }
    }
    let layer = store::StoredLayer::new(
        layer_id.to_string(),
        layer_name.to_string(),
        Some("overpass".to_string()),
        Some("point".to_string()),
        None,
    );
    if let Err(e) = store::add_layer(&conn, &layer) {
        return json!({"error": e.to_string(), "op": "overpass.query"});
    }
    match store::replace_features(&conn, layer_id, &feats) {
        Ok(n) => json!({"layer": layer_id, "ingested": n}),
        Err(e) => json!({"error": e.to_string(), "op": "overpass.query"}),
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
            {"name": "geocode.search", "description": "Search places: query required, limit 1..50, live=false demo else Photon", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "limit": {"type": "integer"}, "live": {"type": "boolean"}}, "required": ["query"]}},
            {"name": "overpass.query", "description": "Overpass tag search: key/value required, bbox south/west/north/east optional, live=false dry-runs QL else executes; ingest=true writes hits to db", "inputSchema": {"type": "object", "properties": {"key": {"type": "string"}, "value": {"type": "string"}, "south": {"type": "number"}, "west": {"type": "number"}, "north": {"type": "number"}, "east": {"type": "number"}, "live": {"type": "boolean"}, "ingest": {"type": "boolean"}, "db": {"type": "string"}}, "required": ["key", "value"]}},
            {"name": "features.list", "description": "List ingested point features, optionally filtered to one layer (db optional)", "inputSchema": {"type": "object", "properties": {"layer_id": {"type": "string"}, "db": {"type": "string"}}}},
            {"name": "export.png", "description": "Export map PNG (Phase 1 render path)", "inputSchema": {"type": "object", "properties": {"width": {"type": "integer"}, "height": {"type": "integer"}}}},
        ]
    })
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn call_tool(name: &str, args: &Value) -> Value {
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
                    &hits,
                );
                return json!({"content": [{"type": "text", "text": json!({"query_result": out, "ingest": stored}).to_string()}]});
            }
            out
        }
        "features.list" => {
            let db = str_arg(args, "db");
            features_list(db.as_deref(), str_arg(args, "layer_id").as_deref())
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
fn run_mcp_stdio() {
    eprintln!("qgiscraft-mcp: MCP server listening on stdio");
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
                Some(call_tool(tname, &targs))
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
                        &hits,
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
        Commands::Mcp => run_mcp_stdio(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_nine_tools() {
        let catalog = tools_list();
        let tools = catalog
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn unknown_tool_becomes_error_content_not_panic() {
        let result = call_tool("nope.missing", &json!({}));
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
        let result = call_tool("layer.add", &json!({"id": "", "name": "x"}));
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
        );
        let text = content_text(&result);
        assert!(text.contains("invalid input"));
    }

    #[test]
    fn overpass_ingest_requires_live() {
        let result = call_tool(
            "overpass.query",
            &json!({"key": "amenity", "value": "pharmacy", "ingest": true}),
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
            &hits,
        );
        assert_eq!(stored.get("ingested").and_then(|v| v.as_u64()), Some(1));
        let listed = features_list(Some(db.as_str()), Some("amenity-pharmacy"));
        assert_eq!(listed.get("count").and_then(|v| v.as_u64()), Some(1));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn geocode_out_of_range_limit_rejected() {
        let result = call_tool("geocode.search", &json!({"query": "Wien", "limit": 99}));
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("limit must be 1..50"));
    }
}
