# docs/gaps.md - ranked work list (v2.3: shared math, chrome, retry, dist)

DONE:
- Core `Command` bus (12 cmds) + validation hardening.
- Store SQLite: layers + features (+geom_json) + visible + counts + migrations + clear_all.
- CLI/MCP `--db` persistence (add/list/remove roundtrip proven).
- MCP stdio: 12 tools, typed schemas. `mcp --connect PORT` forwards the DB-touching tools to the live app; see `docs/control-protocol.md`.
- egui shell: DB-backed project, ear-clipped fills, per-layer palette, slippy zoom (wheel + Ctrl+wheel + pinch), pan, Fit data, F5 refresh.
- Menus wired: Project, Layer, View, Help, Settings, Vector, Raster, Database, Web, Processing (all actions real, no dead buttons); Edit holds a typed placeholder, Plugins holds real tools as they land (Show WKT, Zoom to coordinates first).
- Vector: GeoJSON+CSV import (file picker, validated), GeoJSON export.
- Raster/Web: XYZ tile layers from DB + preset gallery (OSM/HOT/Carto/Esri).
- Database: scratch layers. Processing toolbox dock: search + 3 runnable algorithms + first real geoprocessing (buffer points).
- Geocode: demo + Photon live. Overpass: fenced builder + dry-run + live + `--ingest` (full geometry).
- XYZ tile underlay (cached, progressive), Web Mercator canvas, pull-model BMP preview with fills.
- Ways/polygons/lines with selected-layer labels; fill pixel-proven (`docs/map-preview.png`).
- Identify on hover/click: tooltip with name+coords, click selects the layer and pins details; map picks topmost visible feature (points/lines/polygons), empty click deselects.
- Standard readme stack + plugin strategy (MCP tools are the plugin system) + data ecosystem doc (sources, GTFS et al., where your maps go, walk-and-photo crossconnect).
- Track B seed: Axum sidecar (11254) + React/Leaflet console (11253, proxied) - same command bus over REST, no new capabilities.
- Worked recipes in docs/WORKFLOWS.md (bike-repair-shops walkthrough: tag choice, 3 integration paths, rendered map).
- Oracle rig: QGIS 3.44.15 LTR side-by-side; `plan/qgis/` observations (gitignored); `docs/ui-parity.md`.
- Single projection core (`tiles::project/unproject/fit_view/recenter/meters_per_pixel/nice_scale_bar/layer_color`); app + headless BMP share it, web uses it for fit/tiles.
- Overpass retries (3 attempts, backoff) on 502/503/429/timeouts; public-API flakiness no longer kills a demo.
- Canvas chrome: scale bar (nice 1/2/5 m/km), north arrow, per-layer color dots in dock, live `1:N + bar` readout.
- `just dist`: the three release exes as plain files, a launcher-only `-portable.zip` of the same exes, and `SHA256SUMS.txt` in `dist/`.
- Plugin reimplementations shipped: QuickWKT -> `wkt.show` (paste WKT -> layer + fit, single + multi, 2D), Lat Lon Tools zoom -> Plugins "Zoom to coordinates" (`geo::parse_latlng`, N/S/E/W; copy-coords still open), QGIS "Create grid" -> `processing.grid` (rect/hex, 10k cap), NNJoin -> `processing.nearest` (hub lines with distances), Shape Tools -> `processing.geodesic` (true-distance circle/ellipse/line), single-symbol color -> `layer.color` (#rrggbb per layer, styled in dock/canvas/preview/web), colsto geolocation -> `doppler.locate` MVP (offline curve-vs-track rank -> scored layer; no 920s gap filter yet).

NEXT (in sequence):
1. First oracle pass: canvas nav bindings (wheel/Ctrl+wheel/drag/box), menu inventory, per plan/qgis/02-ui-ux.md.
2. Symbology single/categorized/graduated dialogs (Layer > Styling).
3. KML/GPX import (GeoJSON+CSV done).
4. Clip/Reproject (buffer done; pure Rust, no GDAL).
5. MCP twins for processing (buffer/geocode-download as tools, not just dialogs).
6. Publishing loop: uMap one-click export, then OSM changeset upload (account + review UI first - no unsupervised edits).
7. Field mode: photo attachments on features (walk-and-photograph crossconnect).
8. Print Layout export PNG.
9. OSM Austria PBF bulk loader.
10. Async tile worker (replace 2-fetch/frame sync cap).
11. Unify app + web BMP blit loops (math is shared; only the pixel-pushing differs - low priority).

PLUGIN BACKLOG (capability, not code - per docs/plugins.md each lands as
Command + MCP tool + CLI + real menu action; never vendor a plugin):
Now (tiny, pure math, no new deps - all shipped: QuickWKT, Lat Lon zoom, grid, NNJoin, geodesic, single-symbol color):
- Next shape work is ellipse UX polish only (presets, not new math).
Next (small, still keyless):
- QuickMapServices -> XYZ preset gallery (tile engine exists; only presets + picker missing).
- Spreadsheet Layers -> `layer.import` xlsx via calamine (pure Rust; POI datasets love xlsx).
- ImportPhotos -> `photos.import`: EXIF GPS to point layer (feeds the walk-and-photo crossconnect).
- MBTiles -> tile-cache source (MBTiles is SQLite; we already bundle rusqlite).
Later (needs raster engine or sidecars):
- Profile Tool + keyless SRTM downloader (needs DEM sampler first).
- WhiteboxTools as opt-in sidecar (fits the "GRASS/SAGA via MCP sidecars" slot; nothing GPL in core).
- Routing (ORS et al.): deferred indefinitely - all options need API keys or self-host. Keyless rule wins.
Never: API-keyed panels (MapTiler, StreetView, TravelTime), Mergin/QField sync, PyQGIS compat.
