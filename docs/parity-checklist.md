# docs/parity-checklist.md - presence checklist (`cargo xtask parity`)

Checked = present in the tree as committed. Unchecked = genuinely absent.
Updated 2026-10-24 post-0.6; corrected the v0.1 overclaims (toolbars,
box-zoom, `commands --json`, kittest goldens never existed).

- [x] Menus: Edit holds a typed placeholder ("lands with later phases"); Plugins is real (Show WKT + Zoom to coordinates dialogs)
- [ ] Toolbars: File/Manage Layers/Digitizing/Attributes/Map Navigation (not built; menu bar only)
- [x] Panels: Browser/Layers/Styling/Processing Toolbox/Log (Identify Results panel not built)
- [x] Status bar: project file/CRS/live scale/coords-ish/render state (CRS selector not built)
- [x] Canvas: drag-pan, Ctrl-wheel zoom-to-cursor, Fit data, F5 refresh, point render
- [ ] Box zoom, back/forward extents (not built)
- [ ] Add Vector/Raster/Delimited/PostGIS/WMS dialogs (not built)
- [x] Overpass builder + dry-run + live + ingest (QuickOSM parity, fenced, no raw QL)
- [x] Photon locator via geocode.search (demo + live; no Ctrl+K overlay yet)
- [ ] Attribute table (F6) + Identify (not built)
- [x] `status --json` lists all commands (9 as of 0.3)
- [x] `--smoke` BMP preview (headless map proof; kittest goldens in `cargo test` not built)
- [x] command layer.add present (Layer/Add parity)
- [x] command layer.list present (Layers panel parity)
- [x] command layer.remove present (Layers panel remove parity)
- [x] command geocode.search present (Locator parity)
- [x] command overpass.query present (QuickOSM parity)
- [x] command features.list present (read back ingested hits)
- [x] command layer.import present (GeoJSON/CSV file -> layer parity)
- [x] command processing.buffer present (geoprocessing twin parity)
- [x] command map.forge present (mapforge depot parity: spec -> QL plan -> ingest)
- [x] command wkt.show present (QuickWKT parity: paste WKT -> layer + fit)
- [x] command processing.grid present (Create grid parity: rect/hex over bbox)
- [x] command processing.nearest present (NNJoin parity: hub lines to closest)
- [x] Processing Toolbox: Download, Geocode, Buffer, Create grid, Nearest join (search filters, each opens its dialog)
- [x] Plugins menu: Show WKT + Zoom to coordinates dialogs (first real plugin capabilities)
- [x] forge depot: `depot/maps/*/spec.json` (pharmacies + bikeshops), checked by `cargo run -p xtask -- forge`
- [x] command export.png present (typed unsupported, render path Phase 1+)
