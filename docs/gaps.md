# docs/gaps.md - ranked work list (v0.6: first map render landed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features + visible + counts + migrations.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project, high-contrast theme, Ctrl-wheel zoom-to-cursor, pan, Fit data, F5 refresh.
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest`.
- First light: 29 live pharmacies rendered (`docs/map-preview.png`), preview via `--smoke` BMP.

NEXT (pick from top):
1. Render: lines/polygons from Overpass ways, per-layer colors, labels.
2. Basemap: offline vector tiles (PMTiles Austria) under the points.
3. Symbology single/categorized/graduated dialogs.
4. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
5. Print Layout export PNG.
6. OSM Austria PBF bulk loader.
