# docs/parity-checklist.md - presence checklist (`cargo xtask parity`)

- [x] Menu bar: Project/Edit/View/Layer/Settings/Plugins/Vector/Raster/Database/Web/Processing/Help
- [x] Toolbars: File/Manage Layers/Digitizing/Attributes/Map Navigation
- [x] Panels: Browser/Layers/Styling/Processing Toolbox/Identify/Log
- [x] Status bar: CRS/scale/coords/render toggle
- [x] Canvas: pan/zoom/box-zoom/back-forward (stub bindings, real render Phase 1)
- [ ] Add Vector/Raster/Delimited/PostGIS/WMS dialogs (Phase 0)
- [ ] Overpass builder + Nominatim locator (Phase 0)
- [ ] Attribute table (F6) + Identify (Phase 1)
- [x] `commands --json` lists all commands (via `status --json`)
- [x] `ui.screenshot` golden PNGs in `cargo test`
- [x] command layer.add present (Layer/Add parity)
- [x] command layer.list present (Layers panel parity)
- [x] command layer.remove present (Layers panel remove parity)
- [x] command geocode.search present (Locator parity)
- [x] command overpass.query present (QuickOSM parity)
- [x] command export.png present (Layout export parity)
