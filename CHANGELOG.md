# Changelog - qgiscraft-mcp

## 2026-10-24 - Phase 2.0: control channel (`--control` / `mcp --connect`)
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
