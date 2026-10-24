# docs/gaps.md - ranked work list (v0.2: store + overpass builder landed)

DONE:
- Core `Command` bus + validation hardening.
- Store SQLite: init/add/list/remove roundtrip + `resolve_db_path`/`open_at` + 3->5 col migration.
- CLI/MCP `--db` persistence plumbing (add/list/remove roundtrip proven).
- MCP stdio: initialize/tools-list/tools-call/ping.
- egui shell: real `Project` model (add/remove/visibility/log).
- Geocode: demo default + Photon live (`--live` / `live:true`).
- Overpass: fenced tag builder (injection-fenced) + dry-run QL + live execute (capped 100).

NEXT (pick from top):
1. Photon/Overpass results -> "ingest as layer" (write hits into project DB).
2. Symbology single/categorized/graduated dialogs.
3. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
4. Print Layout export PNG (vello_cpu path like designcraft-render).
5. `.qgz`-equivalent open/save + OSM Austria PBF loader.
6. egui kittest golden PNGs (`ui.screenshot` in `cargo test`).
