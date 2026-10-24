# docs/gaps.md - ranked work list (v0.9: ways + polygons landed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project, high-contrast theme, Ctrl-wheel zoom-to-cursor (slippy), pan, Fit data, F5 refresh.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview.
- Ways/polygons: `out geom`, per-layer palette, line strokes, polygon outlines, selected-layer labels.
- First light with streets: parks + primary roads over tiles (vectors pixel-verified in preview).

NEXT (in sequence):
1. Concave polygon fill (earcut triangulation, not convex hull).
2. Symbology single/categorized/graduated dialogs.
3. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
4. Print Layout export PNG.
5. OSM Austria PBF bulk loader.
6. Async tile worker (replace 2-fetch/frame sync cap).
