# docs/gaps.md - ranked work list (v0.3: ingest + DB-backed app landed)

DONE:
- Core `Command` bus (9 cmds) + validation hardening.
- Store SQLite: layers + features + visible + counts + migrations.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 9 tools, typed schemas.
- egui shell: DB-backed project (open/seed/counts/add/remove/visible persist).
- Geocode: demo + Photon live.
- Overpass: fenced builder + dry-run + live execute + `--ingest` into DB.

NEXT (pick from top):
1. Render path: vello_cpu canvas (points first, then lines/polygons).
2. Symbology single/categorized/graduated dialogs.
3. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
4. Print Layout export PNG.
5. OSM Austria PBF loader (bulk ingest, not just Overpass).
6. egui kittest golden PNGs (`ui.screenshot` in `cargo test`).
