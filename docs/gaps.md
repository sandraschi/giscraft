# docs/gaps.md - ranked work list (v1.0: menus wired)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project, high-contrast theme, Ctrl-wheel zoom-to-cursor (slippy), pan, Fit data, F5 refresh.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview.
- Ways/polygons: `out geom`, per-layer palette, line strokes, polygon outlines, selected-layer labels.
- Menus wired: Project (New armed/Refresh/Quit), Layer (add/remove/zoom-to/show-hide), View (zoom/fit/reset), Help (About); shortcuts F5/Del/Ctrl+Plus/Minus/0.
- First light with streets: parks + primary roads over tiles (vectors pixel-verified in preview).

NEXT (in sequence):
1. Concave polygon fill (earcut triangulation, not convex hull).
2. Symbology single/categorized/graduated dialogs (Layer > Styling).
3. GeoJSON/KML/GPX file import (Layer > Add Vector) + Delimited Text (CSV).
4. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
5. Print Layout export PNG.
6. OSM Austria PBF bulk loader.
7. Async tile worker (replace 2-fetch/frame sync cap).
