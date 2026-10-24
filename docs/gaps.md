# docs/gaps.md - ranked work list (v2.2: toolbox wired + buffer)

DONE:
- Core `Command` bus (10 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations + clear_all.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 10 tools, typed schemas. `mcp --connect PORT` forwards the DB-touching tools to the live app; see `docs/control-protocol.md`.
- egui shell: DB-backed project, ear-clipped fills, per-layer palette, slippy zoom (wheel + Ctrl+wheel + pinch), pan, Fit data, F5 refresh.
- Menus wired: Project, Layer, View, Help, Settings, Vector, Raster, Database, Web, Processing (all actions real, no dead buttons).
- Vector: GeoJSON+CSV import (file picker, validated), GeoJSON export.
- Raster/Web: XYZ tile layers from DB + preset gallery (OSM/HOT/Carto/Esri).
- Database: scratch layers. Processing toolbox dock: search + 3 runnable algorithms + first real geoprocessing (buffer points).
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview with fills.
- Ways/polygons/lines with selected-layer labels; fill pixel-proven (`docs/map-preview.png`).
- Standard readme stack + plugin strategy (MCP tools are the plugin system) + data ecosystem doc (sources, GTFS et al., where your maps go, walk-and-photo crossconnect).
- Track B seed: Axum sidecar (11254) + React/Leaflet console (11253, proxied) — same command bus over REST, no new capabilities.
- Oracle rig: QGIS 3.44.15 LTR side-by-side; `plan/qgis/` observations (gitignored); `docs/ui-parity.md`.

NEXT (in sequence):
1. First oracle pass: canvas nav bindings (wheel/Ctrl+wheel/drag/box), menu inventory, per plan/qgis/02-ui-ux.md.
2. Symbology single/categorized/graduated dialogs (Layer > Styling).
3. KML/GPX import (GeoJSON+CSV done).
4. Clip/Reproject (buffer done; pure Rust, no GDAL).
5. MCP twins for processing (buffer/geocode-download as tools, not just dialogs).
6. Publishing loop: uMap one-click export, then OSM changeset upload (account + review UI first — no unsupervised edits).
7. Field mode: photo attachments on features (walk-and-photograph crossconnect).
8. Print Layout export PNG.
9. OSM Austria PBF bulk loader.
10. Async tile worker (replace 2-fetch/frame sync cap).
11. Unify app + web render paths (one shared `render` module; the BMP math is currently duplicated).
