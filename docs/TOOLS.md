# Tool Reference (9 MCP tools + matching CLI)

Every command is also a CLI subcommand (`--json` for agents). MCP transport:
stdio, MCP 2024-11-05, `initialize` / `tools/list` / `tools/call` / `ping`.

| Tool | Required | Optional | What |
|---|---|---|---|
| `status` | — | — | App mode, command list, agent contract |
| `project.status` | — | — | Project CRS + layer count |
| `layer.add` | `id`, `name` | `source`, `geom_type`, `crs`, `visible`, `db` | Add (upsert) a layer, persists to SQLite |
| `layer.list` | — | `db` | Layers in draw order + `visible` + per-layer feature counts |
| `layer.remove` | `id` | `db` | Remove a layer and its features |
| `geocode.search` | `query` | `limit` 1..50, `live` | Photon demo (Stephansplatz) or live lookup |
| `overpass.query` | `key`, `value` | `south/west/north/east`, `live`, `ingest`, `db` | Fenced tag builder; dry-run QL by default; `live` executes (cap 100); `ingest` writes hits to DB (requires `live`) |
| `features.list` | — | `layer_id`, `db` | Ingested point features (cap 500), GeoJSON in `geom_json` |
| `export.png` | — | `width`, `height` | Typed `unsupported` until the render path lands (Phase 1+) |

CLI equivalents: `status`, `project-status`, `layer-add`, `layer-list`,
`layer-remove`, `geocode-search [--live]`, `overpass-query [--live] [--ingest] [--db]`,
`features-list`, plus `mcp` (serve). Per-command `--db` overrides
`$QGISCRAFT_DB`, default `./qgiscraft.db`.

Hostile input (empty ids, injection in tag keys, out-of-range limits/bbox)
returns typed `invalid input` errors — never a panic, never silent.
