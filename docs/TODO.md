# TODO & Gap Roadmap: qgiscraft-mcp

**Status**: Active Working Backlog  
**Target**: QGIS-class pure-Rust desktop GIS + MCP automation server  
**Strict Safety Gate**: Zero unwraps/panics (`clippy -D warnings`), 100% test green, hermetic tests  

---

## 1. Prioritization Strategy

Our priorities are guided by the repository's core philosophy: **CLI + MCP first, 1:1 QGIS muscle memory second, zero C/C++ dependencies always**.

| Tier | Focus Area | Why This Priority? |
|---|---|---|
| **Tier 1: High Leverage** | MCP Geoprocessing Twins, Async Tiles, GPX Import | Completes the MCP-first promise for automation; eliminates canvas panning stutter; unlocks field GPS data. |
| **Tier 2: Desktop Parity** | Attribute Table (`F6`), Symbology Dialog, Bounding Box Clip | Core QGIS workflows: table inspection, custom visualization styling, and polygon/line clipping. |
| **Tier 3: Scale & Publishing** | Print Layout PNG, uMap Export, OSM PBF Bulk Ingest | Production output generation and large-scale offline country data ingestion. |

---

## 2. Ranked Backlog & Gap Inventory

### Tier 1: Immediate & High Leverage

- [x] **Gap 1: MCP & CLI Processing Twin (`processing.buffer`)** [COMPLETED]
  - *Current*: `processing.buffer` is exposed via MCP tool catalog (11 tools total) and CLI command `--layer-id <ID> --distance <M> --output-layer <ID>`. Forwarded to live app when connected via `--connect`.
  - *Validation*: Headless unit tests pass in core and MCP temp-DB roundtrip.

- [ ] **Gap 2: Asynchronous Tile Worker (Zero UI Stutter)**
  - *Current*: `qgiscraft` fetches up to 2 tiles per UI frame synchronously with a 1.5s timeout. Slow or dropped connections cause micro-stutter during rapid panning.
  - *Action*: Move tile downloads to a decoupled background thread pool or channel (`std::sync::mpsc`). Decoded image buffers stream into an in-memory frame cache.
  - *Validation*: Frame rate stays at locked 60 FPS while panning across cold tile regions.

- [ ] **Gap 3: GPX Vector Importer (`import.rs`)**
  - *Current*: `import.rs` supports GeoJSON and CSV. Field GPS data (Garmin, Strava, OSM traces) comes as `.gpx` XML.
  - *Action*: Implement lightweight, pure-Rust, panic-free GPX parser extracting `<wpt>` (points) and `<trkpt>` (LineStrings). Wire into file picker dialog and CLI/MCP `layer.import`.
  - *Validation*: Test with synthetic GPX trace, rejecting malformed XML without panic.

---

### Tier 2: Desktop Parity & Workflow Depth

- [ ] **Gap 4: Attribute Table Dock (`F6` Muscle Memory)**
  - *Current*: Features are only inspectable via map hover tooltips or CLI `features.list`.
  - *Action*: Pressing `F6` or selecting `Layer > Attribute Table` opens a bottom dock with a scrollable tabular view of the selected layer's features (name, kind, coords, geometry type). Clicking a row highlights the feature on the canvas.
  - *Validation*: UI unit tests in `qgiscraft`.

- [ ] **Gap 5: Symbology & Layer Styling Dock (`Layer > Styling`)**
  - *Current*: Layers use a deterministic 8-color FNV hash palette; users cannot customize colors, point sizes, or line widths.
  - *Action*: In the Layer Styling dock, add controls for point radius, stroke width, fill opacity (0–100%), and color override. Store styles in the SQLite `layers` table or project metadata.
  - *Validation*: Style persistence roundtrip test in `qgiscraft-store`.

- [ ] **Gap 6: Pure-Rust Geometry Clip (`processing.rs`)**
  - *Current*: Only `buffer` is implemented in geoprocessing.
  - *Action*: Implement polygon and linestring bounding box clipping (Cohen-Sutherland for segments, Sutherland-Hodgman for convex polygons) in pure Rust.
  - *Validation*: Test with lines and polygons crossing bounding box boundaries.

---

### Tier 3: Production Output & Scaled Offline Data

- [ ] **Gap 7: Print Layout Export (`export.png`)**
  - *Current*: `export.png` returns a typed error "render path lands Phase 1".
  - *Action*: Extend the headless preview renderer to compose a styled layout: high-resolution map frame + dynamic scale bar + north arrow + layer legend + title header, saved to a PNG file.
  - *Validation*: File output test matching dimension parameters.

- [ ] **Gap 8: Bulk OSM PBF Loader (Offline Country Extracts)**
  - *Current*: Data is downloaded over the network via Overpass API.
  - *Action*: Add pure-Rust OSM PBF reader to ingest regional extracts (`austria-latest.osm.pbf`) directly into the SQLite project DB without internet access.

---

## 3. Definition of Done (Strict Quality Gates)

For each gap closed:
1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
2. `cargo test --workspace` all green (with regression/unit test for every new capability).
3. `cargo fmt --all --check` clean.
4. `cargo run -p xtask -- ci` (includes L6 layer isolation and parity checks).
5. No `unwrap()`, `expect()`, `panic!()`, or `todo!()` anywhere in the codebase.
6. Documentation and `CHANGELOG.md` updated with the concrete improvement.
