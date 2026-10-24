# docs/gaps.md - ranked work list (v0.8: basemap landed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features + visible + counts + migrations.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project, high-contrast theme, Ctrl-wheel zoom-to-cursor (slippy), pan, Fit data, F5 refresh.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest`.
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview.
- First light with streets: 29 Vienna pharmacies over tiles (`docs/map-preview.png`).

NEXT (pick from top, in sequence per 2026-10-24):
1. Ways/polygons: geometry column (WKB/GeoJSON), line/fill render, per-layer colors, labels.
2. Symbology single/categorized/graduated dialogs.
3. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
4. Print Layout export PNG.
5. OSM Austria PBF bulk loader.
6. Async tile worker (replace 2-fetch/frame sync cap).
