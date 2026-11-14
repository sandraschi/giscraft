# GISCraft - Product Requirements Document (PRD)

**Document Identifier:** `CRAFT-PRD-2026-GIS-001`  
**Version:** 1.0.0  
**Status:** Active  
**Domain:** Geographic Information System (GIS), Spatial Analysis, Map Publishing  
**Standard Port:** 14120  
**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  

---

## 1. Executive Summary & Problem Statement

Desktop GIS software (QGIS, ArcGIS) is burdened by heavy C/C++ dependencies (GDAL, PROJ, GEOS), high memory footprints, and complex multi-window dialog traps that impede modern agentic automation and instant cold starts.

**The GISCraft Solution:**
GISCraft is a clean-room, pure-Rust GIS and spatial analysis app with no C/C++ runtime bindings,
driven by one `Command::dispatch` bus shared by the CLI, the MCP stdio server and the desktop app.

**Status rule:** sections 1 and 2 separate what works today from goals. What is actually done is
tracked with proof in [`TIERED_PLAN.md`](TIERED_PLAN.md) and [`gaps.md`](gaps.md).

**Works today (0.2.1):** GeoJSON and CSV import, GeoJSON export, WKT, Overpass and Photon ingest,
an offline city table, point buffer, grid, nearest hub, geodesic shapes, doppler.locate, a 2D
Web Mercator canvas with XYZ tile underlay, per-layer color, identify, Go to place, 18 MCP tools.

---

## 2. Core Functional Requirements (goals, not status)

1. **Spatial Data Engine (`giscraft-core`):**
   - Goal: more formats (GPX, KML, Shapefile, GeoTIFF, MBTiles). Today: GeoJSON, CSV, WKT.
   - Goal: attribute table with filtering and spatial joins. Today: name, kind, geometry per feature.
   - Goal: R-tree index, layer reprojection, clip, Voronoi. Today: linear scans, display-only
     Web Mercator, buffer for points.
2. **Command registry and MCP (`giscraft-mcp`):**
   - Today: one binary with CLI subcommands and a hand-rolled MCP stdio server (not FastMCP).
   - Goal: every processing dialog has an MCP twin; PNG export.
3. **Desktop workstation (`giscraft`):**
   - Today: 2D canvas with pan/zoom, layer dock, identify.
   - Goal: attribute inspector, symbology styling, print layout. A 3D canvas is not planned.

---
## 3. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric GIS Workstation):** Persistent 2D/3D geographic map canvas with dockable layer tree, attribute table drawer, and spatial toolbox. Top quick button row acts as accelerators into the `View` menu. To add a new GIS tool or layer renderer:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Layers / Processing toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New Project (`Ctrl+N`), Open Project / Layer (`Ctrl+O`), Open Recent ▸, Save Project (`Ctrl+S`), Save As... (`Ctrl+Shift+S`), Export Map Image / GeoJSON (`Ctrl+E`), Close (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy, Paste, Select All (`Ctrl+A`), Preferences (`Ctrl+,`).
  - `View`: Map Canvas Viewport, Layer Tree Panel, Attribute Table (`F6`), Processing Toolbox, CRS Status Bar, Theme (`Dark`/`Light`).
  - `Insert`: Add Vector Layer, Add Raster Layer, Add WMS/XYZ Tile Stream, Create New Shapefile.
  - `Modify`: Edit Feature Geometry, Calculate Attributes, Change Layer Symbology, Reproject Layer (CRS).
  - `Commands`: Run Buffer Analysis, Spatial Join, Clip Vector Overlay, Generate Contour Lines.
  - `Window`: Reset Layout, Map Viewport, Layer Manager Drawer, Processing Dock.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About GISCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
