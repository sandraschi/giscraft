# docs/target-app-parity.md - QGIS parity assessment (stub v0.1)

Target: QGIS Desktop (LTR) as observed black-box. Goal: 5-minute switch for
daily users. Muscle memory outranks novelty. No backend cruft: modern Rust
geo stack, everything is a `Command` reachable via CLI + MCP + control channel.

## 1. Shell parity (must clone)

- Menu bar: Project, Edit, View, Layer, Settings, Plugins, Vector, Raster, Database, Web, Processing, Help - same order, same shortcuts (Ctrl+O, Ctrl+S, Ctrl+Shift+V/A/R, F6 attribute table, Ctrl+K locator).
- Toolbars: File, Manage Layers, Digitizing, Advanced Digitizing, Attributes, Map Navigation - same icon grouping, tooltips with shortcuts.
- Panels (dockable): Browser, Layers, Layer Styling, Processing Toolbox, Identify Results, Log Messages. Same default layout as fresh QGIS profile.
- Status bar: CRS selector, scale, magnifier, coordinates (map units), render toggle.
- Canvas: pan (space/middle-drag), zoom wheel, box zoom, back/forward extents.

## 2. Data parity (Phase 0)

- Add Vector (GeoPackage, Shapefile, GeoJSON, KML, GPX), Add Raster (GeoTIFF, XYZ tiles), Add Delimited Text (CSV XY), Add PostGIS/WMS/WMTS (dialog parity, connect/test/new).
- OSM harness: QuickOSM-style Overpass query builder (key=value presets: amenity=pharmacy, tourism=museum, leisure=playground), Nominatim/Photon locator, Austria PBF offline default.
- Save/open `.qgz` equivalent: SQLite project store (layers, styles, extents, CRS).

## 3. Analysis parity (Phase 1+)

- Processing Toolbox subset first: Buffer, Clip, Dissolve, Reproject, Hillshade, Contour. Same param dialogs, batch mode later.
- Symbology: Single symbol, Categorized, Graduated, Rule-based (subset), label engine basic.
- Print Layout: title, map frame, legend, scalebar, north arrow, export PNG/PDF/SVG (via `designcraft`-style export path).

## 4. Non-goals (explicitly not cloned)

- Python plugin runtime (replaced by MCP commands), Qt4-era dialogs, GRASS/SAGA native embedding (via MCP sidecars later), 3D globe (Phase 2+).

## 5. Oracle loop

Original app is the oracle: click and drag, write the file out, diff the clone. Same for UI: screenshot original, screenshot clone, diff frames. Start at one object (one point layer), grow to thousands. See `standards/rust-artcraft/ORACLE_LOOP.md`.
