# GISCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(giscraft-core)        (giscraft-mcp)        (giscraft-app)        (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Spatial Engine & Geometry Ops (`giscraft-core`)
- [x] L1 `Command::dispatch` universal GIS execution bus.
- [x] Vector geometry representations (points, linestrings, polygons).
- [x] Spatial indexing and bounding box queries.
- [x] Coordinate reference system handling (WGS84 EPSG:4326, Web Mercator EPSG:3857).
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`giscraft-mcp`)
- [x] Unified CLI `giscraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] GeoJSON and spatial data file export/import.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`giscraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation.
- [x] GPU-accelerated interactive 2D map canvas with pan/zoom.
- [x] Layer panel tree, attribute table inspector, and symbology controls.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About GISCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
