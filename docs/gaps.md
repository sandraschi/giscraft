# docs/gaps.md - ranked work list (v1.2: zoom fixed, oracle installed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations + clear_all.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas. `mcp --connect PORT` forwards the DB-touching
tools to the live app (single writer); see `docs/control-protocol.md`.
- egui shell: DB-backed project, ear-clipped fills, per-layer palette, slippy zoom (wheel + Ctrl+wheel + pinch), pan, Fit data, F5 refresh.
- Menus wired: Project (New armed/Refresh/Quit), Layer (add/remove/zoom-to/show-hide), View (zoom/fit/reset), Help (About); shortcuts.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview with fills.
- Ways/polygons/lines with selected-layer labels; fill pixel-proven (`docs/map-preview.png`).
- Standard readme stack: README/INSTALL/llms/glama + full `docs/` (WRAPPEE, plugins strategy, CONFIG, DEV, ARCH, TOOLS, TROUBLE, ONBOARDING, index); plugin answer = MCP tools are the plugin system.
- Oracle rig: QGIS 3.44.15 LTR side-by-side; `plan/qgis/` observations (gitignored); `docs/ui-parity.md`.

NEXT (in sequence):
1. First oracle pass: canvas nav bindings (wheel/Ctrl-wheel/drag/box), menu inventory, per plan/qgis/02-ui-ux.md.
2. Symbology single/categorized/graduated dialogs (Layer > Styling).
3. GeoJSON/KML/GPX file import (Layer > Add Vector) + Delimited Text (CSV).
4. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
5. Print Layout export PNG.
6. OSM Austria PBF bulk loader.
7. Async tile worker (replace 2-fetch/frame sync cap).
