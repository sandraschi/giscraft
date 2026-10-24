# docs/gaps.md - ranked work list (v1.1: concave fill landed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations + clear_all.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project, high-contrast theme, ear-clipped polygon fill, per-layer palette, Ctrl-wheel zoom-to-cursor (slippy), pan, Fit data, F5 refresh.
- Menus wired: Project (New armed/Refresh/Quit), Layer (add/remove/zoom-to/show-hide), View (zoom/fit/reset), Help (About); shortcuts.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview with fills.
- Ways/polygons/lines with selected-layer labels; fill pixel-proven (`docs/map-preview.png`).

NEXT (in sequence):
1. Symbology single/categorized/graduated dialogs (Layer > Styling).
2. GeoJSON/KML/GPX file import (Layer > Add Vector) + Delimited Text (CSV).
3. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
4. Print Layout export PNG.
5. OSM Austria PBF bulk loader.
6. Async tile worker (replace 2-fetch/frame sync cap).
