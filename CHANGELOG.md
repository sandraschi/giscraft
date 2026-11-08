# Changelog - qgiscraft

## Unreleased - app binary renamed qgiscraft-app -> qgiscraft, repo hygiene
- Native app crate + binary are now `qgiscraft` (dir `crates/qgiscraft`): `cargo run -p qgiscraft`, `qgiscraft.exe`. justfile (run-app, run-app-smoke, dist), start.ps1, and live docs updated; dated assessments keep the old name as history.
- `.gitattributes` enforces LF (`*.bat` CRLF); whole tree normalized from CRLF. Em dashes replaced with hyphens repo-wide. AGENTS.md gains section 6 (LF, single trunk, no em dash).
- Fixed pre-existing gate breakage from the i18n commit: clippy single_element_loop on the Edit menu + fmt drift. Gates: fmt, clippy -D warnings, 133 tests, app --smoke all green.

## Unreleased - plugins round 2: geodesic + layer.color + doppler.locate (18 tools)
- New `processing.geodesic` tool + CLI: true-distance circle/ellipse/line (geodesic vertices via `destination_point`, densified XY-to-line), persists via `ingest_hits`. Completes the Now plugin backlog (QuickWKT, coords, grid, NNJoin, geodesic all shipped).
- New `layer.color` tool + CLI + app styling picker: per-layer `#rrggbb` persisted in a new `layers.color` column (migration included, empty = palette default), honored by canvas, headless BMP, web preview, and `/api/layers`. Styling panel: picker for the selected layer + reset-to-palette; `layer.color` also works over MCP connect (control arm refreshes the live app).
- New `doppler.locate` MVP (colsto method): offline curve-vs-track matcher - sound-travel iteration, terrain-corrected range, normalized shape correlation, grid rank to top-N scored points. Caveats documented: no 920s gap negative-evidence filter yet, single-capture JSON in, emission clamped to track ends.
- Gates: core 80 + mcp 28 green (geodesic/doppler hostile tests, dispatch + persist + stdio probes for grid/geodesic/doppler); full workspace link still blocked by the running app exe - close/reopen to pick up the new build.

## Unreleased - processing.grid + processing.nearest (grid hexagons, NNJoin hub lines)
- New `processing.grid` tool + CLI (15 tools total): rect/hex grids over a bbox (10k cell cap, geodesic hex vertices via `destination_point`, auto-closed rings), persists polygons; dry-run is the compute (no live needed). New `processing.nearest` tool + CLI: hub LineStrings from each source feature to its closest target (haversine, 1M pair cap, "A -> B (312 m)" names), persists lines. Backed by pure `processing::grid_features` / `nearest_join` + dispatch validation; L2/app own persistence.
- Toolbox: "Create grid" (prefills canvas bbox, hex checkbox) + "Nearest neighbor join" (source prefilled from selection) dialogs; both persist + auto-fit + log like the other algorithms.
- Gates: core 70 + mcp 22 green, fmt clean, xtask parity 13 + forge green, MCP stdio probe (wkt.show earlier; grid/nearest via CLI live runs). Full workspace link blocked again by the running app exe - close/reopen to pick up the new build.

## Unreleased - plugins go real: wkt.show + zoom-to-coords (QuickWKT/LatLon Tools)
- New `wkt.show` tool + `wkt-show` CLI (13 tools total): hand-rolled 2D WKT reader (POINT/LINESTRING/POLYGON + MULTI, SRID prefix tolerated, auto-closed rings, EMPTY -> zero features), persists via `ingest_hits` with shared geom-type sniff. Plugins menu: real "Show WKT..." + "Zoom to coordinates..." dialogs (lat,lng + N/S/E/W suffixes via `geo::parse_latlng`).
- Filed `docs/gaps.md` plugin backlog (capability, not code): Now (QuickWKT done, coords done, grid, NNJoin, geodesic), Next (QMS presets, xlsx, ImportPhotos, MBTiles), Later (profile/SRTM, Whitebox sidecar), Never (API-keyed, Mergin, PyQGIS).
- Gates: core 64 + mcp 18 green (parse/coord hostile tests, dispatch + persist tests), fmt clean; full workspace clippy/test re-run pending app restart (running exe locks the linker).


## Unreleased - repo rename qgiscraft-mcp -> qgiscraft (craft convention)
- GitHub `sandraschi/qgiscraft-mcp` renamed to `sandraschi/qgiscraft` (old URL redirects); local checkout moves to `D:\Dev\repos\qgiscraft`. Crate/binary names unchanged (`qgiscraft-mcp` L2 CLI/MCP, `qgiscraft-app`, `qgiscraft-web`): only the repo container, clone URLs, titles, and `glama.json` were renamed, like `gamecraft` (repo) vs `gamecraft-mcp` (crate).
- Gates re-verified after rename: clippy -D warnings, tests green, fmt clean.

## Unreleased - mapforge MVP (spec -> QL plan -> ingest, depot + forge gate)
- New `map.forge` tool + `forge-from-spec` CLI (12 tools total): MapSpec JSON (title/slug/bbox/1..8 layers) validates offline, dry-run returns deterministic Overpass QL per layer, `--live` executes (cap 100/layer), `--ingest` persists each layer to SQLite. Single content envelope over MCP (no double-wrap), hostile inputs are typed errors.
- New `crates/qgiscraft-core/src/mapforge.rs`: `parse_spec` + `compile_plan` (slug charset, 120-char titles, Bbox fence, duplicate ids, injection via `build_tag_query`), 3 unit tests.
- Depot: `depot/maps/innere-stadt-pharmacies/spec.json` + `depot/maps/innere-stadt-bikeshops/spec.json` (29-hit WORKFLOWS pair), verified by CLI dry-run + stdio probe.
- `xtask forge`: depot presence gate (required keys per spec), wired into `ci`; `parity` now covers 10 commands (added `features.list`, `layer.import`, `processing.buffer`, `map.forge`).
- Gates: 94 tests green (55 core + 16 mcp + 13 app + 8 store + 2 web), clippy -D warnings, fmt clean.

## Unreleased - doppler SPEC + hygiene (bak cleanup, gitignore both patterns)
- New `docs/SPEC-doppler-locate.md`: offline-first `doppler.locate` design (curves + tracks layer -> scored layer, no audio/FFT in core, no live ADS-B in v1, colsto 920s-pair method with sound-travel + terrain note).
- Hygiene: removed 4 stale `*.bak` from 2026-10-25 batch (app/main, core/lib, mcp/main, docs/TOOLS); `.gitignore` now covers both `*.bak` and `*.bak.*` per fleet gate.
- Gates: 87 tests green, clippy -D warnings, fmt clean.

## Unreleased - identify on hover/click, high-contrast dots, POI presets, glossary
- MCP twin for geoprocessing: new `processing.buffer` tool and `processing-buffer` CLI command (11 tools total); forwarded to live app via control channel with real-time canvas refresh (87 tests green).
- Hovering the map shows a name+kind+coords tooltip; clicking selects the layer and pins details; empty click deselects (QGIS Identify behavior).
- Picking lives in core (`point_in_ring`, `dist_to_segment_deg`), topmost-visible-wins, 12px tolerance; covered by unit + app tests.
- Dots: white halo underlay so they read on busy tiles; per-layer colors already differed (verified in render).
- Removed the meaningless "Add demo layer" buttons; Layer menu links to real import, dock has "Add OSM data...".
- OSM download dialog has one-click POI presets (Cafes, Pharmacies, Bike shops, Restaurants, Bars, ATMs, Locksmiths, Bakeries, Supermarkets).
- New `docs/GLOSSARY.md` (bbox, Overpass, tiles, CRS, ODbL...) linked from README + docs index.

## 2026-10-24 - Phase 2.5: worked recipe (bike shops) + basemap hardening
- New `docs/WORKFLOWS.md`: end-to-end "Innere Stadt bike repair shops" recipe (tag choice with `craft=bicycle_repair` zero-hit lesson, 3 integration paths, rendered map `docs/map-bikeshops.png`).
- Hardened basemap selection: raster layers need a *usable* template (`is_usable_template`: http(s) + {z}/{x}/{y}); legacy `xyz:osm` stubs no longer shadow the default with a broken URL. Seed writes full template.
- Gates green: 82 tests, clippy -D warnings, fmt, xtask ci.
- Single projection core: `tiles::{project, unproject, fit_view, recenter, meters_per_pixel, nice_scale_bar, layer_color}` with offline tests; app canvas/preview and web server use it (doubles as the fix for the degrees-vs-world01 fit bug class).
- Canvas chrome: scale bar (nice 1/2/5 m/km), north arrow, per-layer color dots in the Layers dock, live `1:N + bar` readout.
- Overpass retries (3 attempts, 2s/4s backoff) on 502/503/429/timeouts.
- `just dist`: naked-PC zip (release exes + README/INSTALL/CHANGELOG + START-HERE.txt).
- Gates green: 81 tests, clippy -D warnings, fmt, xtask ci.
- `qgiscraft-web` Axum sidecar on 11254 (Track B seed): GET /api/health|layers|features|geocode|overpass|preview.png, CORS open, serves `web/dist` when built.
- Map console (`web/`, Vite+React+Leaflet, dev on 11253 proxied): Leaflet map + live layers sidebar, geocode demo/live, one-click Overpass ingest + fit, server-rendered preview link.
- No new capability: same dispatch + store calls as MCP (duplicated thin glue; unify behind one service layer later - noted in gaps).
- Gates green: 75 tests + `cargo build` web crate + `vite build` clean (console serves at `/`, API verified live: layers, geocode, 585KB tiled preview.png).
- Fixed two real bugs found by looking: async-context panic (blocking reqwest inside handler now runs on `spawn_blocking`) and preview fit in degrees instead of world01 (continent view -> street view).

## 2026-10-24 - Phase 2.2: toolbox wired + buffer
- Processing Toolbox dock is real: search field + 3 runnable algorithms (OSM download, Geocode, Buffer points); dead Buffer/Clip/Dissolve buttons removed.
- New `processing.rs`: great-circle destination + `buffer_features` (circle polygons, closed rings, typed errors), 3 tests incl. C-shape area preservation pattern.
- Vector/Raster/Database/Web/Processing menus fully wired (import/export, XYZ layers + preset gallery, scratch layers, download/geocode dialogs).
- Gates green: 73 tests, clippy -D warnings, fmt, xtask ci.
- Vector: import GeoJSON/CSV via file picker (validated, auto-detected), export selected layer as GeoJSON. New core `import.rs` (parse + export, 6 tests).
- Raster: XYZ Tile Layer dialog (template validated); canvas renders the first visible raster layer, OSM default otherwise.
- Web: XYZ preset gallery (OSM Standard/Humanitarian, Carto Dark, Esri Imagery) - one click swaps the basemap.
- Database: New Scratch Layer dialog (point/line/polygon), QGIS equivalent.
- Processing: OSM download dialog (key/value/use-view, worker thread, auto-ingest+fit) + Geocode-to-layer dialog.
- New `layer.import` command + MCP tool + CLI (10/10). rfd native dialogs.
- Gates green: 70 tests, clippy -D warnings, fmt, xtask ci.
- App serves JSON-lines on 127.0.0.1 (`--control PORT`, `QGISCRAFT_CONTROL_PORT`): 7 methods (project.status, layer.add/list/remove, ui.inspect, map.fit, map.render), error+close on garbage, 4MB cap, 16-conn cap, no auth.
- `mcp --connect PORT` forwards the 4 DB-touching tools to the live app (single writer); network tools stay local.
- `core::control` owns framing/validation/client; app owns execution; live TCP tests (stub executor + garbage-closes-connection).
- `docs/control-protocol.md` mirrors the storytold contract.
- Gates green: 61 tests, clippy -D warnings, fmt, xtask ci.
- No grey on black: full-white body text (`override_text_color`), widget strokes, canvas frame, attribution; near-black surfaces kept.
- Settings menu: Configuration window shows effective endpoints (DB, tiles template, Photon, Overpass) + env override names; new `QGISCRAFT_{TILES,PHOTON,OVERPASS}_URL` overrides in core (offline-tested template expansion).
- Gates green: 56 tests, clippy -D warnings, fmt, xtask ci.

## 2026-10-24 - Phase 1.2: zoom that works + QGIS oracle installed
- Fixed dead Ctrl-wheel zoom: egui routes Ctrl+scroll into `zoom_delta()` and empties the scroll channel by design; now reads the factor path AND plain-wheel (QGIS default) smoothed deltas. Found by reading egui 0.37 wheel_state source, not by guessing.
- QGIS 3.44.15-Solothurn LTR installed side-by-side (`C:\Program Files\QGIS 3.44.15`, `qgis-ltr.bat`).
- Oracle rig: `plan/qgis/` (gitignored, local-only) with `02-ui-ux.md` observation checklist; `docs/ui-parity.md` starter scorecard, honest zero.
- Gates green: 54 tests, clippy -D warnings, fmt, xtask ci.
- Hand-rolled ear clipping in core (`triangulate.rs`, no new deps, panic-free, tested: square/triangle/C-shape area/concave/degenerate).
- Canvas: `Mesh` fill (35% alpha) + outline; preview: half-space scanline fill blended over tiles.
- Fill proven by pixel test (exact blended bytes at view center), not just eyeballing.
- Gates green: 54 tests, clippy -D warnings, fmt, xtask ci.
- Project menu: New (double-click armed wipe + reseed), Refresh, Quit.
- Layer menu: Add demo layer, Remove selected, Zoom to selected, Show/Hide all.
- View menu: Zoom in/out, Fit data, Reset view (Wien z15). F5/Del/Ctrl+Plus/Minus/0 shortcuts.
- Help > About window (version, OSM attribution, workflow hint).
- `clear_all` store fn + test; zoom-to-layer via `fit_view(only=)`.
- Gates green: 48 tests, clippy -D warnings, fmt, xtask ci.
- Overpass `out geom`: ways distill to LineString/Polygon (+centroid); nodes stay points. `element_to_hit` pure + tested.
- Store `geom_json` column (GeoJSON, 7->8 col migration); ingest passes geometry through; points keep None.
- Canvas: per-layer palette (FNV hash, 8 high-contrast colors), line strokes, polygon outlines (no fill yet - concave needs earcut), selected-layer name labels.
- Preview draws the same geometry (DDA lines/rings); first verified render: parks + primary roads over tiles.
- Gates green: 46 tests, clippy -D warnings, fmt, xtask ci.
- XYZ raster tiles (OSM, cache-first disk cache, max 2 fetches/frame, progressive paint-in) under vector points.
- Web Mercator throughout: canvas, fit, zoom (slippy 2..19), status shows z + 1:scale.
- Pull-model BMP preview (no more subpixel grid); OSM attribution on canvas + docs.
- First light with streets: 29 Vienna pharmacies over Innere Stadt tiles, `docs/map-preview.png`.
- Gates green: 39 tests (5 new tile math), clippy -D warnings, fmt, xtask ci.

## 2026-10-24 - Phase 0.7: hygiene batch (assessment findings)
- Env rename `QGISCRRAFT_DB` -> `QGISCRAFT_DB` (was misspelled since 0.1; fixed before anything depended on it).
- `parity-checklist.md` rewritten honestly: toolbars/box-zoom/F6/WMS dialogs unchecked, Overpass+Photon checked, `--smoke` BMP instead of kittest claim.
- `AGENTS.md` section 2 corrected: core deps + live I/O, L2 owns persistence (intentional seam, keeps dispatch tests hermetic).
- `.github/workflows/ci.yml`: Windows fmt+clippy+test+xtask.
- Gates green: 34 tests, clippy -D warnings, fmt, xtask ci, smoke with renamed env.

## 2026-10-24 - Phase 0.6: first map render
- Canvas draws ingested points (cyan, selected layer orange, off-canvas culled).
- Ctrl-wheel zooms to cursor, drag pans, Fit data frames visible layers, F5 refreshes from DB.
- Fit zoom range fixed 0.25..8 -> 0.25..64 with 20% padding (city bboxes no longer render tiny).
- Shared `fit_view` between canvas and headless BMP preview; `--smoke` writes `qgiscraft-preview-<pid>.bmp`.
- First light: 29 live Vienna pharmacies, see `docs/map-preview.png`.
- Gates green: 34 tests (6 app incl. projection/fit/BMP), clippy -D warnings, fmt, xtask ci.
- High-contrast dark visuals (near-black surfaces, bright selection/text).
- Ctrl+wheel zooms the canvas (0.25x-8x); status bar shows live `1:denom (pct%)` + project file name.
- Canvas has a zoom-scaled crosshair anchor + frame so zoom is visible pre-render.
- Gates green: 28 tests, clippy -D warnings, fmt, smoke.

## 2026-10-24 - Phase 0.4: GUI proven + live end-to-end
- `qgiscraft-app --smoke`: headless open/seed/list proof (no window, CI-safe); `just run-app-smoke`.
- Live verified by hand: Photon `Stephansplatz` -> 3 real hits; Overpass `amenity=pharmacy` Innere Stadt -> 29 ingested, read back via `features.list` and `layer.list` (features: 29).
- Gates green: 28 tests, clippy -D warnings, fmt, xtask ci.

## 2026-10-24 - Phase 0.3: ingest + DB-backed app
- Overpass `--ingest` writes live hits into the project DB (`db` param, slug layer id, replace semantics); dry-run + ingest is a typed error.
- New `features.list` tool + CLI (9 tools total); `layer.list` carries `visible` + per-layer feature counts.
- Store: `features` table, `visible` column, `set_visible`/`replace_features`/`list_features`/`feature_counts`, 3->7 col migration path.
- egui app owns the DB file directly: open/seed/counts/add/remove/visible persist; the DB file IS the project (`.qgz` equivalent).
- Tests hermetic via temp DBs: 28 green. Gates green: clippy -D warnings, fmt, xtask ci/layers/parity, stdio probe.

## 2026-10-24 - Phase 0.2: SQLite persistence + Overpass builder
- `--db` plumbing: CLI + MCP layer add/list/remove persist to SQLite (`$QGISCRRAFT_DB` or `./qgiscraft.db`); roundtrip proven via CLI smoke.
- Store 3->5 col migration (geom_type, crs) with duplicate-column-tolerant init; name validation; `resolve_db_path`/`open_at`.
- Overpass: fenced tag builder (quotes/semicolons/control rejected) + dry-run QL + live execute capped at 100 hits.
- Tests hermetic via unique temp DBs (no env mutation): 24 tests green.
- Gates green: clippy -D warnings, fmt, xtask ci/layers/parity, stdio probe.

## 2026-10-24 - Phase 0.1: real project model + Photon live path
- App owns a real `Project` (add/remove/visibility/select, typed log, 50-line cap).
- Store gains `remove_layer` + `default_db_path`/`open_default` (`QGISCRRAFT_DB` override); CLI/MCP->DB wiring deferred to keep tests hermetic.
- Geocode: `geocode.rs` demo default + `photon_search` live (`--live` CLI, `live` MCP param, 12s timeout, fenced parse).
- Gates green: clippy -D warnings, 19 tests, fmt, xtask ci/layers/parity, stdio probe.

## 2026-10-24 - Scaffold
- Pure-Rust ArtCraft workspace created (core/store/mcp/app + xtask).
- QGIS parity target locked: 1:1 clone for muscle memory, no backend cruft.
- Ports 11253/11254 claimed for Track B web parallel (egui native needs zero ports).
- Gates green: clippy -D warnings, 15 tests, fmt, xtask ci/layers/parity, stdio probe initialize/tools-list/tools-call.
