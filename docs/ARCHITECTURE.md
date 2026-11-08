# Architecture

```
qgiscraft (eframe/egui, Track A) ─┐ in-process core calls
                                       ├─► qgiscraft-core (L1): Command bus, geo/validation,
qgiscraft-mcp CLI ─── dispatch ────────┤     Photon/Overpass clients, tiles, earcut
                                       │
qgiscraft-mcp MCP (stdio) ─────────────┘
        │  hand-rolled JSON-RPC (MCP 2024-11-05), stdout pure
        ▼
qgiscraft-store (SQLite): layers + features(+geom_json) + visible
        ▲ DB file IS the project (our .qgz); default ./qgiscraft.db
```

- **L1 `qgiscraft-core`**: geo types, project model, `Command::dispatch`
  (10 ids), live Photon/Overpass clients (opt-in), slippy tile math + fetch,
  ear-clipping triangulation. No UI, no MCP framing.
- **Store**: rusqlite/bundled, migrations tolerant to old schemas (3->8 cols).
- **L2 `qgiscraft-mcp`**: CLI-first binary + MCP stdio server. Owns SQLite
  persistence (`layer_add/list/remove`, `ingest_hits`, `features_list`);
  core dispatch stays hermetic so its tests need no DB.
- **L3 `qgiscraft`**: egui native shell, QGIS muscle layout. Owns the DB
  file directly (open/seed/list/add/remove/visible persist). Web Mercator
  canvas, XYZ tiles (2 fetches/frame, disk cache), `--smoke` headless
  proof + BMP preview. `--control PORT` serves the same state over JSON-lines.
- **Track B `qgiscraft-web` + `web/` console**: thin Axum sidecar (11254)
  re-exposing dispatch + store as REST (`/api/health|layers|features|geocode|
  overpass|preview.png`, CORS open); React+Leaflet console on 11253 (Vite dev,
  proxied) or served from `web/dist`. No new capability - same bus, HTTP-shaped.
- **Transports**: stdio MCP (line-delimited JSON-RPC; logs on stderr,
  pure JSON on stdout); HTTP sidecar for humans/demos; control channel for
  live-app driving.
- **Clean room**: QGIS is observed black-box only (`plan/qgis/`,
  never committed). No QGIS code, icons, or artwork in this repo.
