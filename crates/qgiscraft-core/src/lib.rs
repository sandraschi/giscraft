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
pub mod overpass;
pub mod project;
pub mod tiles;
pub mod triangulate;

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
                CMD_GEOCODE_SEARCH, CMD_OVERPASS_QUERY, CMD_FEATURES_LIST, CMD_EXPORT_PNG,
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
        assert_eq!(cmds.len(), 9);
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

    #[test]
    fn dispatch_future_is_send() {
        fn assert_send<T: Send>(_: T) {}
        assert_send(dispatch(Command::Status));
    }
}
