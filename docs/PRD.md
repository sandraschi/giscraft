# GISCraft — Product Requirements Document (PRD)

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
GISCraft is a clean-room, pure-Rust Geographic Information System and spatial analysis atelier:
- Permissive pure-Rust spatial crates without C/C++ runtime bindings.
- Multi-layer vector and raster map canvas with GPU tile rendering (`egui` / `wgpu`).
- Spatial indexing (R-tree), attribute queries, vector geometric overlays (buffer, clip, convex hull), and CRS re-projection.
- Universal `Command::dispatch` execution bus driving CLI, FastMCP stdio server, and native desktop app.

---

## 2. Core Functional Requirements

1. **Spatial Data Engine (`giscraft-core`):**
   - Layer management (vector GeoJSON, Shapefile, raster GeoTIFF, MBTiles).
   - Attribute table storage with SQL filtering and spatial joins.
   - Geometry analysis algorithms (Haversine distance, polygon buffering, Voronoi tessellation).
2. **Command Registry & FastMCP (`giscraft-mcp`):**
   - Unified `CommandSpec` registry driving CLI subcommands, MCP tools, and UI dialogs.
3. **Desktop Workstation (`giscraft-app`):**
   - Interactive 2D/3D map canvas with pan/zoom navigation, attribute inspector, and symbology styling.

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
