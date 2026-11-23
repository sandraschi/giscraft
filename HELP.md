# GISCraft Comprehensive Manual & Spatial Intelligence Handbook

**GISCraft Local-First Spatial Intelligence Engine & QGIS Clean-Room Shell**  
**Version**: 0.2.1  
**Domain**: Geographic Information Systems (GIS), Geodesy, OpenStreetMap/Overpass Harvesting, Vector Spatial Processing, Doppler Emitter Localization  
**Architecture**: 5-Crate Pure-Rust Workspace (`giscraft`, `giscraft-core`, `giscraft-store`, `giscraft-mcp`, `giscraft-web`)  
**Control Port / Track B**: Native SQLite Project Track (zero ports) | Optional `--control PORT` (`127.0.0.1`) | Reserved Web Track B (`11253` / `11254`)

---

## 1. What GISCraft Is & Does + Get Started

### What GISCraft Is
**GISCraft** is a local-first, clean-room 1:1 **QGIS** shell and spatial processing engine written in pure Rust. Traditional QGIS is a large C++ / Qt / Python suite (GDAL, PROJ, GEOS, SpatiaLite) that is heavy to embed and awkward for headless AI automation. GISCraft preserves QGIS muscle memory—menus, shortcuts (`F5`, `F6`, `Ctrl+G`, `Ctrl+0`), Browser & Layers docks, and interactive map canvas—while unifying the desktop app, CLI, and MCP `stdio` server on a single typed command bus backed by a self-contained SQLite project database (`giscraft.db`).

All workspace crates enforce `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, and `#![deny(clippy::panic)]`, and require zero API keys or accounts (Photon geocoding, Overpass API, and OSM XYZ basemap tiles are keyless).

### 5-Step Get Started Tour
1. **Launch with Seeded Basemap**: Start the desktop workstation with `cargo run -p giscraft --` (or launch `giscraft.exe`). If `giscraft.db` is empty, GISCraft automatically seeds an `osm-basemap` XYZ tile layer centered on Vienna (`48.2082° N, 16.3738° E`, zoom `15.0`).
2. **Navigate & Geocode (`Ctrl+G`)**: Drag to pan, scroll to zoom at cursor, or press `Ctrl+G` (**View -> Go to place...**) to jump to any city in the 15-city offline index or query Photon live. You can also enter coordinates directly in `lat, lng` format (e.g., `48.2082, 16.3738`).
3. **Ingest or Import Data**:
   - **Live OSM**: Open **Processing -> Download OSM features...**, enter a tag pair such as `amenity` = `pharmacy`, and ingest features within the current viewport bounding box.
   - **Local Files**: Use **Layer -> Add Vector Layer...** (`.geojson`) or **Layer -> Add Delimited Text Layer...** (`.csv` with `lng`/`lat` columns).
4. **Analyze & Inspect (`F6`, Processing, `Ctrl+J`)**: Run geodesic buffers, rectangular/hex grids, nearest-neighbor hub lines, or WKT previews. Press `F6` to open the **Attribute Table** dock—clicking any row highlights that feature on the canvas. Press `Ctrl+J` to open the built-in **AI Spatial Advisor** chat dock.
5. **Export & Share**: Select a vector layer in the Layers dock and choose **Vector -> Export Layer as GeoJSON...** to export RFC 7946 GeoJSON for uMap, GitHub, QGIS, or downstream tools.

---

## 2. Functions

GISCraft is organized into five workspace crates sharing one domain model and command bus:

| Crate | Role | Key Responsibilities |
| :--- | :--- | :--- |
| **`giscraft-core`** | Pure-Rust GIS Engine | Coordinate validation (`geo.rs`), Web Mercator math (`tiles.rs`), ear-clipping polygon triangulation (`triangulate.rs`), WKT 2D parser (`wkt.rs`), GeoJSON/CSV import & export (`import.rs`), fenced Overpass QL builder (`overpass.rs`), Photon geocoding (`geocode.rs`), geodesic buffers/grids/nearest-neighbor (`processing.rs`), Doppler localization (`doppler.rs`), MapSpec forge (`mapforge.rs`), JSON-lines control framing (`control.rs`), LLM chat & tool execution (`llm.rs`), embedded skills (`skills.rs`), and i18n (`i18n.rs`). |
| **`giscraft-store`** | SQLite Project Store | Single-file `.qgz` equivalent (`giscraft.db`). Transactional layer and feature persistence, draw-order visibility, per-layer styling colors, and feature replacement. |
| **`giscraft-mcp`** | CLI & MCP `stdio` Server | Exposes all 18 spatial tools over MCP 2026-07-28 (`initialize`, `tools/list`, `tools/call`, `ping`) and matching `--json` CLI subcommands. Supports `--connect PORT` to drive a live GUI instance. |
| **`giscraft`** | Native Desktop Studio | Immediate-mode `egui 0.37` / `eframe` workstation with QGIS-style menu bar, Browser & Layers docks, off-thread XYZ tile worker (`tile_worker.rs`), vector canvas (`canvas_tiles.rs`), attribute table (`attribute_table.rs`), LLM chat dock (`chat_dock.rs`), and Help Center (`help_view.rs`). |
| **`giscraft-web`** | HTTP Sidecar (Track B) | Thin `axum` REST + static web console seed sharing the same `giscraft-core` and `giscraft-store` primitives. |

### Core Spatial Capabilities
- **Keyless Geocoding (`geocode.search`)**: Offline table of 15 major cities for instant zero-network lookup, plus live Photon API search (`--live`).
- **Fenced Overpass Engine (`overpass.query`, `map.forge`)**: Validates tag keys/values against QL injection, builds dry-run Overpass QL by default, and executes live queries (capped at 100 elements per query) with automatic retry on `429`/`502`/`503`.
- **Geodesic & Vector Processing**:
  - `processing.buffer`: Great-circle buffer around point features (`distance` in meters, `segments` `8..=256`).
  - `processing.grid`: Rectangular or pointy-top hexagonal (`hex: true`) spatial grid over a `south/west/north/east` bbox (cap `10,000` cells).
  - `processing.nearest`: Computes closest target feature for each source feature and emits hub linestrings (cap `1,000,000` pairs).
  - `processing.geodesic`: Generates true-distance geodesic circles, ellipses (major/minor axes + bearing), or great-circle lines (cap `20,000 km`).
  - `wkt.show`: Parses 2D `POINT`, `LINESTRING`, `POLYGON`, and `MULTI*` WKT strings and persists them as a map layer.
- **Doppler Emitter Localization (`doppler.locate`)**: Matches measured Doppler frequency-shift curves against ADS-B aircraft/satellite tracks over a candidate grid to rank probable emitter coordinates.

---

## 3. Usage (GUI, CLI & MCP)

### Desktop GUI Controls & Shortcuts
| Action | Shortcut / Menu | Behavior |
| :--- | :--- | :--- |
| **Pan / Zoom** | Left-drag / Mouse wheel | Smooth pan; wheel zooms centered at cursor (`zoom` `2.0..=19.0`). |
| **Zoom In / Out** | `Ctrl+=` / `Ctrl+-` | Step zoom in or out; also under **View**. |
| **Fit Data** | `Ctrl+0` | Fits viewport to all visible vector layers. |
| **Go to Place** | `Ctrl+G` | Opens geocoder / `lat, lng` jump dialog. |
| **Refresh** | `F5` | Reloads layers and features from `giscraft.db`. |
| **Attribute Table** | `F6` | Toggles bottom attribute dock; clicking a row highlights the feature. |
| **Remove Selected Layer** | `Delete` | Deletes the currently selected layer and its features. |
| **AI Spatial Chat Dock** | `Ctrl+J` | Toggles the right-hand LLM advisor with confirmation gates for mutating tools. |
| **Command Palette** | `Ctrl+Shift+P` or `Ctrl+K` | Fuzzy-search and execute any menu command or tool immediately. |
| **Keyboard Shortcuts** | `F1` or `Ctrl+/` | Opens the categorized hotkey reference modal. |

### CLI Workflows (`giscraft-mcp --json`)
Every MCP tool has a matching CLI subcommand. Use `--db PATH` to target a specific SQLite project (overrides `$GISCRAFT_DB`, default `./giscraft.db`).

```powershell
# 1. Inspect project CRS and layer inventory
giscraft-mcp project-status --json
giscraft-mcp layer-list --json

# 2. Geocode offline (fast) or live via Photon
giscraft-mcp geocode-search --query "Stephansplatz" --json
giscraft-mcp geocode-search --query "Praterstern Vienna" --live --json

# 3. Dry-run Overpass QL first, then execute live and ingest into SQLite
giscraft-mcp overpass-query --key amenity --value pharmacy --south 48.20 --west 16.36 --north 48.22 --east 16.39 --json
giscraft-mcp overpass-query --key amenity --value pharmacy --south 48.20 --west 16.36 --north 48.22 --east 16.39 --live --ingest --json

# 4. Run spatial processing and style the output layer
giscraft-mcp processing-buffer --layer-id osm-amenity-pharmacy --distance 250 --segments 32 --output-layer pharmacy-250m --json
giscraft-mcp processing-grid --south 48.20 --west 16.36 --north 48.22 --east 16.39 --cell-size 300 --hex --output-layer hex-grid --json
giscraft-mcp layer-color --id pharmacy-250m --color "#e11d48" --json
```

### MCP `stdio` Server & Live GUI Bridge
Configure your MCP client (Claude Desktop, Cursor, Antigravity) to launch `giscraft-mcp mcp`.

- **Headless Mode**: `giscraft-mcp mcp` reads/writes `giscraft.db` directly.
- **Live GUI Bridge (Single-Writer Safe)**: Start the desktop app with `giscraft -- --control 19250`, then run `giscraft-mcp mcp --connect 19250`. All DB-mutating MCP calls are forwarded over `127.0.0.1` JSON-lines to the GUI thread so the canvas updates live without SQLite lock contention.

**Available MCP Tools (18)**: `status`, `project.status`, `layer.add`, `layer.list`, `layer.import`, `layer.remove`, `layer.color`, `features.list`, `geocode.search`, `overpass.query`, `processing.buffer`, `processing.grid`, `processing.nearest`, `processing.geodesic`, `map.forge`, `wkt.show`, `doppler.locate`, `export.png` (currently typed `unsupported`).

---

## 4. Tips & Tricks

- **Always Dry-Run Overpass & Forge First**: `overpass.query` and `map.forge` default to `live: false`, returning the generated Overpass QL without touching the network. Verify your bounding box and tag filter before passing `--live --ingest`.
- **Use `--connect PORT` for Interactive Agent Sessions**: Instead of closing the GUI to let an agent edit `giscraft.db`, launch `giscraft -- --control 19250` and attach `giscraft-mcp mcp --connect 19250`. Every `layer.add`, `processing.*`, and `layer.color` call renders immediately on screen.
- **Instant Offline City Jumps**: `Ctrl+G` and `geocode.search` resolve 15 major world cities offline with zero latency—ideal for demos, air-gapped environments, and smoke tests.
- **Row-Click Feature Highlighting**: Press `F6` to open the Attribute Table, then click any row to highlight that feature on the map canvas and verify its coordinates/kind.
- **Custom XYZ Basemaps**: Use **Web** presets for one-click basemap switching (OSM Standard, CartoDB, etc.) or **Raster -> Add XYZ Tile Layer...** for custom `{z}/{x}/{y}` tile servers.
- **Environment Overrides**: Set `GISCRAFT_DB`, `GISCRAFT_TILES_URL`, `GISCRAFT_PHOTON_URL`, or `GISCRAFT_OVERPASS_URL` before launch to point at local tile caches or self-hosted Overpass/Photon instances.

---

## 5. Common Pitfalls & Safety

### 1. CRS / EPSG Projection Mismatches
- **The Pitfall**: GISCraft stores all project coordinates in **WGS84 (`EPSG:4326`)** angular degrees and projects on-the-fly to **Web Mercator (`EPSG:3857`)** for tile and vector rendering (clamped at `±85.0511°` latitude). If you import a GeoJSON or CSV exported in a projected CRS (such as `EPSG:3857` meters or UTM Zone 33N `EPSG:32633`, where coordinates look like `1822740, 6141550`), coordinates will exceed `[-180..180]` / `[-90..90]` and fail validation—or worse, if small, plot at Null Island `(0, 0)`.
- **Safety Gate**: `giscraft-core::import` and `geo::require_lon_lat` reject `NaN`, `±inf`, `|lat| > 90`, and `|lng| > 180` with typed `invalid input` errors. Always reproject external files to `EPSG:4326` (e.g., `ogr2ogr -f GeoJSON -t_srs EPSG:4326 out.geojson in.shp`) before importing.

### 2. WGS84 `lat, lng` vs. GeoJSON / WKT `[lng, lat]` Order
- **The Pitfall**: Human GPS readouts, Google Maps URLs, and `Ctrl+G` (`parse_latlng`) use **`lat, lng`** (e.g., Vienna `48.2082, 16.3738`). By contrast, **RFC 7946 GeoJSON**, **WKT** (`POINT(16.3738 48.2082)`), and `LonLat::new(lng, lat)` use **`[lng, lat]`** (`X, Y` Cartesian order). Swapping them places Vienna (`16.37° N, 48.21° E`) in the Gulf of Aden, or triggers an immediate latitude-range error for coordinates above `90° E` (like Tokyo `139.7° E`).
- **Safety Gate**: CSV import inspects header names (`lng`/`lon`/`long`/`x` and `lat`/`y`) rather than assuming column order, and `geo::parse_latlng` accepts explicit hemisphere suffixes (`48.2082N, 16.3738E`) to prevent silent axis flips.

### 3. Large GeoJSON, CSV & Shapefile Memory Bounds
- **The Pitfall**: Loading multi-hundred-megabyte GeoJSON/Shapefile extracts into an immediate-mode desktop canvas or SQLite transaction can exhaust RAM and freeze the UI thread. Additionally, Esri Shapefiles (`.shp`/`.dbf`/`.shx`) are not natively parsed by GISCraft.
- **Hard Safety Caps in `giscraft-core`**:
  - **GeoJSON & CSV Import**: Capped at **`10,000`** features/rows per file (`cap 10000`). Split or spatially clip large extracts before importing.
  - **Spatial Grids (`processing.grid`)**: Capped at **`10,000`** cells. Increase `cell_size` or shrink the bounding box if rejected.
  - **Nearest-Neighbor (`processing.nearest`)**: Capped at **`1,000,000`** source×target pairs.
  - **Geodesic Lines (`processing.geodesic`)**: Capped at **`20,000 km`**.
  - **Overpass Live Queries**: Capped at **`100`** elements per query; **Feature List** capped at **`500`** features.
  - **Control Socket (`--control`)**: Localhost `127.0.0.1` only, **`4 MiB`** max line length, **`16`** max concurrent connections.

### 4. Licensing, Attribution & Single-Writer Discipline
- **ODbL & Tile Etiquette**: OpenStreetMap data and tiles are licensed under **ODbL** (`(c) OpenStreetMap contributors`). Never copy proprietary map data (e.g., Google Maps) into OSM. Avoid hammering public Overpass endpoints—use dry-run mode and tight bounding boxes.
- **Single-Writer SQLite**: Never run a headless `giscraft-mcp` write command against `giscraft.db` while the desktop GUI has the same file open without `--connect PORT`; use the `--control` bridge so the GUI remains the sole SQLite writer.

---

## 6. Planned Extensions

- **Headless Cartographic Export (`export.png`)**: Wire the off-screen renderer into `giscraft-mcp` so `export.png` produces high-DPI PNG/BMP maps with scale bars and north arrows (currently returns typed `unsupported`).
- **GeoTIFF & DEM Hillshading**: Native pure-Rust raster elevation parsing for terrain shading, slope analysis, and contour line extraction.
- **GPX & KML Track Ingestion**: Direct import of GPS hiking logs, waypoints, and flight/drone telemetry tracks.
- **GeoPackage (`.gpkg`) & PBF Bulk Loader**: Offline OGC GeoPackage read/write and Geofabrik `.osm.pbf` regional filtering without external GDAL/C dependencies.
- **TravelCraft Chrono-GIS Bridge & Track B Web Console**: Historical map overlays (e.g., Baedeker chronomaps) and completion of `giscraft-web` on reserved ports `11253`/`11254`.

---

## 7. Other Crafts

GISCraft is part of the **Storytold Craft** pure-Rust workstation family. Each Craft app ships a unified binary/workspace with headless CLI + MCP `stdio` parity and `Command` bus dispatch:

| Craft App | Domain | Relationship / Handoff with GISCraft |
| :--- | :--- | :--- |
| **`travelcraft`** | Travel Planning & Chrono-Cartography | Consumes geocoded POIs, GeoJSON routes, and historical map layers from GISCraft for itinerary & Baedeker chronomap views. |
| **`cadcraft`** | 2D/3D CAD & Technical Drafting | Complementary precision drafting; GISCraft handles geodetic `EPSG:4326`/`3857` coordinates while CADCraft handles local millimeter/architectural plans. |
| **`vectorcraft`** | Vector Illustration & SVG Studio | Receives exported map outlines/polygons (GeoJSON -> SVG) for editorial cartography and print styling. |
| **`celcraft` / `anicraft`** | 2D Animation & Exposure Sheets | Sibling reference implementations of the Storytold Craft Menu Bar & Help Center standard. |
| **`democraft`** | Automated Demo & Screen Capture | Drives `giscraft-mcp` / `--smoke` recipes (`geocode-layer`, `overpass-fenced`, `render-preview`) for reproducible walkthroughs. |
