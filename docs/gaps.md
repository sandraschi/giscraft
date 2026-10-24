# docs/gaps.md - ranked work list (v0.1, pick from top)

1. Core `Command` bus: status, layer.add/list/remove, project.status - DONE scaffold, needs GeoJSON validation hardening.
2. Store SQLite: init/add/list roundtrip - DONE scaffold, needs CRS + style columns.
3. MCP stdio: initialize/tools-list/tools-call/ping - DONE scaffold, needs JSON Schema hardening.
4. egui shell: menu/docks/canvas/status parity skeleton - DONE scaffold, needs kittest goldens.
5. Geocode search offline demo + Photon live path (env-gated, typed errors).
6. Overpass QL builder + minimal result ingest (node/way -> layer).
7. Symbology single/categorized/graduated dialogs.
8. Processing Buffer/Clip/Reproject (pure Rust, no GDAL yet).
9. Print Layout export PNG (vello_cpu path like designcraft-render).
10. `.qgz`-equivalent open/save + OSM Austria PBF loader.
