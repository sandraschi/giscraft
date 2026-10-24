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
    /// Add a layer (id/name/source/geom-type)
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
        #[arg(long)]
        json: bool,
    },
    /// List layers (Phase 0: in-memory note)
    LayerList {
        #[arg(long)]
        json: bool,
    },
    /// Remove a layer by id
    LayerRemove {
        #[arg(long)]
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Geocode search (Phase 0 demo: Vienna; live Photon lands Phase 0.1)
    GeocodeSearch {
        #[arg(long)]
        query: String,
        #[arg(long)]
        limit: Option<u8>,
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
            {"name": "layer.add", "description": "Add a layer: id, name required; source, geom_type, crs optional", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "name": {"type": "string"}, "source": {"type": "string"}, "geom_type": {"type": "string"}, "crs": {"type": "string"}}, "required": ["id", "name"]}},
            {"name": "layer.list", "description": "List layers in draw order", "inputSchema": {"type": "object", "properties": {}}},
            {"name": "layer.remove", "description": "Remove a layer by id", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}},
            {"name": "geocode.search", "description": "Search places: query required, limit 1..50", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "limit": {"type": "integer"}}, "required": ["query"]}},
            {"name": "overpass.query", "description": "Overpass QL query (Phase 0.1 live; now typed unsupported)", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}},
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
            let id = str_arg(args, "id").unwrap_or_default();
            let lname = str_arg(args, "name").unwrap_or_default();
            run(Command::LayerAdd {
                id,
                name: lname,
                source: str_arg(args, "source").unwrap_or_else(|| "osm".to_string()),
                geom_type: str_arg(args, "geom_type").unwrap_or_else(|| "point".to_string()),
                crs: str_arg(args, "crs"),
            })
        }
        "layer.list" => run(Command::LayerList),
        "layer.remove" => run(Command::LayerRemove {
            id: str_arg(args, "id").unwrap_or_default(),
        }),
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
                })
            }
        }
        "overpass.query" => run(Command::OverpassQuery {
            query: str_arg(args, "query").unwrap_or_default(),
        }),
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
            json,
        } => show(
            run(Command::LayerAdd {
                id,
                name,
                source,
                geom_type,
                crs,
            }),
            json,
            "layer.add",
        ),
        Commands::LayerList { json } => show(run(Command::LayerList), json, "layers"),
        Commands::LayerRemove { id, json } => {
            show(run(Command::LayerRemove { id }), json, "layer.remove");
        }
        Commands::GeocodeSearch { query, limit, json } => show(
            run(Command::GeocodeSearch { query, limit }),
            json,
            "geocode.search",
        ),
        Commands::Mcp => run_mcp_stdio(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_eight_tools() {
        let catalog = tools_list();
        let tools = catalog
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(tools.len(), 8);
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
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|i| i.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        assert!(text.contains("invalid input"));
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
