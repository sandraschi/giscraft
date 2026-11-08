# GISCraft Comprehensive Manual & Spatial Intelligence Handbook

**GISCraft Local-First Spatial Intelligence Engine & QGIS Clean-Room Implementation**  
**Version**: 0.2.0  
**Domain**: Geographic Information Systems (GIS), Geodesy, Chrono-Cartography, Overpass/OSM Harvesting, Doppler Signal Triangulation, Vector Spatial Processing  
**Control Port**: `11254` (Backend + MCP HTTP) / `11253` (Web Interface) / Pure Native SQLite Project Track

---

## 1. What Is GIS? (Geographic Information Systems)

A **Geographic Information System (GIS)** is a computational framework for capturing, storing, querying, analyzing, and visualizing spatial and geographic data. 

At its core, GIS links **location** (where things are on Earth) with **attributes** (what things are like):
1. **The Vector Model**: Represents discrete world entities using exact geometry:
   - **Points**: Zero-dimensional coordinate pairs $(\text{lng}, \text{lat})$ marking POIs (pharmacies, radio towers, mountain peaks).
   - **Linestrings**: One-dimensional ordered vertex arrays representing continuous paths (roads, rivers, hiking trails, flight corridors).
   - **Polygons**: Two-dimensional closed linear rings enclosing areas (national borders, administrative districts, lakes, building footprints).
2. **The Raster Model**: Represents continuous space as grid cells / pixels (satellite imagery, elevation heatmaps/DEMs, radar returns, and tiled map basemaps like OpenStreetMap Web Mercator `EPSG:3857`).
3. **Coordinate Reference Systems (CRS)**: Mathematical models transforming the irregular, 3D gravitational geoid of planet Earth onto a flat 2D viewport.
   - **EPSG:4326 (WGS 84)**: Standard GPS spherical coordinates in angular degrees $(\text{latitude}, \text{longitude})$.
   - **EPSG:3857 (Web Mercator)**: Conformal projection used by modern web maps and GISCraft that renders tiles as perfect squares while preserving angles at local scale.

---

## 2. What Is QGIS?

**[QGIS](https://qgis.org)** (formerly *Quantum GIS*, founded in 2002 by Gary Sherman) is the world’s leading open-source desktop Geographic Information System. 

It is the open-source counterpart to proprietary commercial cartography software (chiefly **Esri ArcGIS Desktop / ArcGIS Pro**, which costs \$1,500 to \$10,000+ per seat per year). QGIS is maintained by an international volunteer foundation and is renowned for its standard layout:
- **Menu Bar**: `Project`, `Edit`, `View`, `Layer`, `Settings`, `Plugins`, `Vector`, `Raster`, `Processing`, `Help`.
- **Docks**: Left-hand Browser tree and Layers list with visibility toggles and z-index ordering.
- **Interactive Canvas**: Pan-and-zoom map viewport with projection engine, scale bar, and coordinate readout.
- **Processing Toolbox**: Hundreds of geometric algorithms (buffers, convex hulls, Voronoi diagrams, Delaunay triangulations).

### Who Uses GIS and QGIS?
- **Urban Planners & Civil Engineers**: Zoning laws, public transport line planning, utility pipeline routing.
- **Environmental Scientists & Foresters**: Deforestation monitoring, flood zone risk modeling, wildlife corridor tracking.
- **Disaster Response & Humanitarian Aid (HOT OSM)**: Earthquake damage assessment, evacuation corridor routing.
- **Intelligence & SIGINT Analysts**: Doppler radio transmitter localization, satellite reconnaissance, logistics interception.
- **Hikers, Explorers & Historians**: Historical route cartography, offline waypoint reconnaissance (e.g. Baedeker chronomaps in TravelCraft).

---

## 3. What Is GISCraft?

**GISCraft** is a local-first, clean-room 1:1 QGIS shell implementation written from the ground up in pure **Rust**.

### Why GISCraft?
Traditional QGIS is a massive C++ / Qt5 / Python codebase spanning over 20 years of legacy dependencies (GDAL, PROJ, GEOS, PyQGIS, SpatiaLite). It is heavy to embed, slow to spin up headlessly, and difficult for autonomous AI agents to drive cleanly.

**GISCraft solves this by decoupling the QGIS experience into a modern local-first architecture:**
1. **Preserves QGIS Muscle Memory**: Retains the iconic menus, shortcuts (`Ctrl+Z`, `Ctrl+Shift+K`, `F5`, `Ctrl+Alt+S`), left-hand Browser & Layers docks, and canvas behavior.
2. **Pure-Rust Zero-Panic Core**: Built with `#![deny(clippy::unwrap_used)]` zero-panic containment.
3. **Database-as-Project (`.qgz` equivalent)**: The entire project state (layers, styles, geometries, CRS, caches) is stored in a single local SQLite database file (`giscraft.db`), eliminating fragmented project files.
4. **Agentic AI & MCP-First**: Every single UI action corresponds directly to a typed variant on the `Command` bus. An AI assistant (via Anthropic/DeepMind MCP stdio) can geocode places, execute live Overpass queries, compute geodesic buffers, and export maps with the exact same authority as a human clicking the mouse.

---

## 4. Current Architecture & Existing Features

GISCraft is structured into a modular multi-tier workspace:
- **`crates/giscraft-core`**: The foundational GIS engine, spatial indexing, coordinate projection (EPSG:4326 $\leftrightarrow$ EPSG:3857), Overpass QL parser, and command bus.
- **`crates/giscraft-store`**: SQLite-backed feature and layer storage with transactional commits.
- **`crates/giscraft-mcp`**: High-performance stdio MCP server for agentic interaction and CLI (`giscraft-mcp`).
- **`crates/giscraft`**: Native immediate-mode desktop studio built in Rust using `egui 0.37` and `eframe`.

### Currently Implemented Features:
1. **Keyless Geocoding**:
   - Photon API integration + offline embedded city index (instant lookup of Stephansplatz, Karlsplatz, etc.).
2. **OpenStreetMap Overpass Engine**:
   - Live query generator and executor for nodes, ways, and relations.
   - Live ingestion: Query amenities (e.g., `amenity=pharmacy`, `highway=cycleway`) and instantly materialize them as vector layers.
3. **High-Performance Vector Canvas**:
   - Real-time XYZ raster tile caching (OpenStreetMap, CartoDB Positron, Stamen Toner).
   - True polygon triangulation: ear-clipping rendering for complex concave boundaries.
   - Point marker styling with customizable colors, halos, and selection rings.
4. **Comprehensive Vector & Processing Toolbox**:
   - **Geodesic Shapes**: Accurate great-circle circles, ellipses, and arcs using WGS-84 ellipsoidal math.
   - **Grid Generation**: Rectangular bounding-box grids and hexagonal spatial tessellations.
   - **Nearest Neighbor Analysis**: Computes geodesic distances and vectors between layers.
   - **Euclidean Buffering**: Buffer zones around points, lines, and polygons.
   - **Well-Known Text (WKT)**: Direct import, visualization, and editing of WKT geometries.
5. **SIGINT / Doppler Emitter Localization (`doppler.locate`)**:
   - High-order Doppler frequency shift curves mapped against satellite/aircraft flyovers to pinpoint terrestrial radio emitters down to geographic coordinates.
6. **Cartographic Export**:
   - Headless PNG and BMP map rendering (`export.png`) with scale bars, north arrows, and alpha layer blending.

---

## 5. Planned Capabilities & Roadmap

1. **TravelCraft & GISCraft Bridge (Chrono-GIS)**:
   - Direct spatial ingestion into TravelCraft’s Baedeker 2030 engine.
   - Historical map vector overlays (e.g. Edo-period Tokyo roads juxtaposed over modern Shinjuku GPS coordinates).
2. **GeoTIFF & Digital Elevation Model (DEM) Hillshading**:
   - Native raster elevation parsing to render 3D terrain shadows and contour lines directly on the egui canvas.
3. **GPX & KML Track Analysis**:
   - Live telemetry ingestion for outdoor hiking tracks, heart-rate GPX logs, and drone flight telemetry.
4. **Offline SpatiaLite / GeoPackage (`.gpkg`) Parity**:
   - Direct binary OGC GeoPackage read/write support without requiring external GDAL dynamic C libraries.
5. **Vector Tile Specification (MVT / Protobuf)**:
   - Client-side vector tile styling via Mapbox GL JSON style specifications.

---

## 6. How To Use GISCraft

### Desktop GUI Navigation
- **Pan**: Left-click and drag anywhere on the canvas.
- **Zoom**: Scroll wheel (centered at mouse cursor) or pinch-to-zoom.
- **Fit Full View (`Ctrl+Shift+F`)**: Zooms canvas to encompass all active vector layers.
- **Refresh (`F5`)**: Clears and re-fetches tile caches and invalidates stale geometries.
- **Layer Control**: Click the checkbox next to any layer in the left dock to toggle visibility.
- **Add Vector Layer**: Use `Vector -> Add Vector Layer...` to import any `.geojson` or `.csv` file.

### Agentic MCP Tool Commands
When connected via Claude Desktop, Antigravity, or the CLI:

```bash
# Check status and loaded layers
giscraft-mcp status --json

# Geocode an address
giscraft-mcp geocode-search --query "Stephansplatz Vienna" --json

# Harvest OpenStreetMap features into a new layer
giscraft-mcp overpass-query --key amenity --value pharmacy --ingest --layer-id pharmacies_vienna --json

# Create geodesic search ring around a target coordinate
giscraft-mcp processing-geodesic --shape circle --lng 16.3738 --lat 48.2082 --radius-m 2500 --layer-id perimeter_2500m --json

# Export high-res cartographic render
giscraft-mcp export-png --output map.png --width 1920 --height 1080 --json
```
