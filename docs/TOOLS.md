# Tool Reference (13 MCP tools + matching CLI)

`mcp --connect PORT` drives the running app (`qgiscraft-app -- --control PORT`)
for the DB-touching tools, so there is a single writer. See
[control-protocol.md](control-protocol.md). Without `--connect` everything
runs headless against the project DB file.

Every command is also a CLI subcommand (`--json` for agents). MCP transport:
stdio, MCP 2024-11-05, `initialize` / `tools/list` / `tools/call` / `ping`.

| Tool | Required | Optional | What |
|---|---|---|---|
| `status` | — | — | App mode, command list, agent contract |
| `project.status` | — | — | Project CRS + layer count |
| `layer.add` | `id`, `name` | `source`, `geom_type`, `crs`, `visible`, `db` | Add (upsert) a layer, persists to SQLite |
| `layer.list` | — | `db` | Layers in draw order + `visible` + per-layer feature counts |
| `layer.import` | `path` | `format` (auto\|geojson\|csv), `layer_id`, `db` | GeoJSON/CSV file -> layer (auto-detected, validated, ingested) |
| `layer.remove` | `id` | `db` | Remove a layer and its features |
| `geocode.search` | `query` | `limit` 1..50, `live` | Photon demo (Stephansplatz) or live lookup |
| `overpass.query` | `key`, `value` | `south/west/north/east`, `live`, `ingest`, `db` | Fenced tag builder; dry-run QL by default; `live` executes (cap 100); `ingest` writes hits to DB (requires `live`) |
| `features.list` | — | `layer_id`, `db` | Ingested point features (cap 500), GeoJSON in `geom_json` |
| `processing.buffer` | `layer_id` | `distance` (meters, def 200), `segments` (8..256), `output_layer`, `db` | Great-circle buffer point features into circle polygons; persists to DB |
| `map.forge` | `path` | `live`, `ingest`, `db` | MapSpec JSON file -> offline QL plan per layer; `live` executes (cap 100/layer); `ingest` writes live hits to DB (requires `live`) |
| `wkt.show` | `wkt` | `layer_id`, `db` | WKT string (POINT/LINESTRING/POLYGON + MULTI, 2D only) as a layer; persists to DB |
| `export.png` | — | `width`, `height` | Typed `unsupported` until the render path lands (Phase 1+) |

CLI equivalents: `status`, `project-status`, `layer-add`, `layer-list`, `layer-import`,
`layer-remove`, `geocode-search [--live]`, `overpass-query [--live] [--ingest] [--db]`,
`features-list`, `processing-buffer`, `forge-from-spec --path SPEC [--live] [--ingest] [--db]`, `wkt-show --wkt GEOM [--layer-id] [--db]`, plus `mcp` (serve, `--connect` for the live app). Per-command `--db` overrides
`$QGISCRAFT_DB`, default `./qgiscraft.db`. Forge depot `depot/maps/*/spec.json` checked by `cargo run -p xtask -- forge`.

Hostile input (empty ids, injection in tag keys, out-of-range limits/bbox)
returns typed `invalid input` errors — never a panic, never silent.
