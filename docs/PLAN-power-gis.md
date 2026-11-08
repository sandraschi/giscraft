# PLAN: giscraft as a powerful GIS for all user levels

Status: proposed 2026-11-08. Supersedes the ordering in `docs/gaps.md` and
`docs/TODO.md` (both stay as history; their open items are folded in below).

## Where we actually are (2026-11-08)

- 18 commands, CLI + MCP + egui app + thin web console. Gates green (133 tests).
- **Data model is the ceiling.** A feature is `name, kind, lng, lat, geom_json`.
  There are **no attributes**: GeoJSON properties and CSV columns are dropped on
  import. No attribute table, filter, categorized style, field calculator or
  join is possible until this changes.
- No spatial index (every query is a full scan), no CRS handling beyond
  EPSG:4326 in / Web Mercator display, no raster, no export except GeoJSON.
- Geometry ops are hand-rolled per tool (buffer = points only, no clip, no
  overlay). Fine for the first plugins, does not scale to a toolbox.
- Two dispatch paths: core `dispatch` still returns Phase-0 stubs for
  `project.status` / `layer.list`; L2 overrides them. MCP tool schemas are
  hand-written JSON in an 80 KB `giscraft-mcp/src/main.rs`, duplicated by clap
  definitions and app dialogs. Every new tool costs three hand edits.

## Principles (unchanged, restated)

- Pure Rust, no GDAL/PROJ/GEOS C libs, nothing GPL. Permissive crates only
  (MIT/Apache/BSD), license checked per crate when added.
- Keyless network only. Clean-room vs QGIS (black-box oracle, never code).
- Every capability = one registry entry -> CLI + MCP + app dialog + web route,
  with a headless test. No dead buttons; unfinished things are typed
  `unsupported`, never faked.
- AGENTS.md gates (zero unwrap/panic, clippy -D warnings, fmt, tests, xtask ci).

## User levels and what each needs

| Level | Needs | Delivered by |
|---|---|---|
| Beginner (never used GIS) | "Put my spreadsheet / GPS track / OSM places on a map, make it pretty, export it" | Task wizards on a start page, smart defaults, plain-language help, auto styling |
| QGIS user | Muscle memory: attribute table F6, identify, toolbox, symbology, layouts, digitizing, CRS | Phases 2, 4, 6 |
| Power user / analyst | Overlay, spatial joins, statistics, raster/terrain, batch, reproducible pipelines | Phases 3, 5, 6 |
| Agents (MCP/CLI) | Small discoverable tool surface, typed params, paging, dry-runs, live-app control | Phase 0 registry, `processing.run`, `pipeline.run` |

## Phase 0 - Foundations (prerequisite for everything) ~1.5 days

1. **Command registry** (`giscraft-core::registry`): one table of
   `CommandSpec { id, summary, params: [ParamSpec{name, type, unit, range,
   default, required}], group, mutates_db }`. Generates MCP `tools/list`
   schemas, clap help/arg parsing (generic `--param` mapping), `commands --json`,
   `describe <id>` (CRAFT_COMMAND_BUS standard), TOOLS.md (xtask), and the
   app's generic tool dialog. Split `giscraft-mcp/src/main.rs` into modules.
2. **Single dispatch path**: kill the Phase-0 stubs; persistence moves behind a
   `Store` handle passed into dispatch so core, CLI, MCP, app, web share it.
3. **Feature model v2** (migration, old DBs keep working):
   - `features.props_json` (JSON object), stable `fid`, geometry always present
     (points get `geom_json` too), per-layer `fields` table (name, type:
     int/real/text/bool/date, inferred on import, editable).
   - Layer metadata: `extent`, `style_json`, `filter_expr`, `group`, `order`,
     `opacity`, `min/max_scale`, `source_crs`.
   - SQLite R*Tree index on feature bboxes (bundled SQLite has the module).
4. **Geometry engine**: adopt `geo` + `geo-types` + `geojson` (+ `wkb`);
   wrap existing tested code where it is already right (geodesic, grid, WKT).
5. **CRS**: `proj4rs` + built-in EPSG table for the codes people actually use
   (4326, 3857, UTM zones, ETRS89-LAEA 3035, MGI Austria 31254-31256/31287,
   national grids as needed). Store 4326 internally, reproject at the edges
   (import/export/measure), metric ops in a local projection.
6. Tests: migration from today's `giscraft.db`, registry round-trip, every
   existing tool still green.

## Phase 1 - Data in and out ~2 days

- Import (all keep attributes + CRS): GeoJSON, CSV/TSV (lat/lon or WKT column
  autodetect), **XLSX** (`calamine`), **GPX** (`gpx`), **KML/KMZ** (`kml`),
  **Shapefile** (`shapefile`, .dbf attrs, .prj -> CRS), **GeoPackage** vector
  (it is SQLite: rusqlite + GPB/WKB decode), **FlatGeobuf** (`flatgeobuf`),
  **photos** (EXIF GPS -> points, `kamadak-exif`).
- Offline bulk: **OSM PBF** (`osmpbf`) with tag filter + bbox (austria-latest).
- Export: GeoJSON, CSV(+WKT), XLSX, KML, GPX, Shapefile, GeoPackage, FlatGeobuf.
- Tools: `layer.import` (formats extended), `layer.export`, `layer.info`
  (fields, extent, count, CRS), `formats.list`, `osm.import_pbf`.
- Live sources (keyless, existing pattern): Overpass gains full tag
  attributes; Photon results keep properties.

## Phase 2 - Attribute engine ~2 days

- **Expression language** (QGIS-like subset, hand-rolled Pratt parser, depth
  and size bounded, hostile-safe): fields, literals, arithmetic, comparisons,
  AND/OR/NOT, LIKE/ILIKE, IN, IS NULL, CASE, and functions (`$area`,
  `$length`, `$x`, `$y`, `round`, `upper`, `lower`, `concat`, `coalesce`,
  `to_int`, `to_real`, `length`, `substr`, `regexp_match`, `now`, ...).
  Geodesic `$area`/`$length` by default.
- Tools: `features.query` (filter, order_by, limit/offset paging, fields
  subset, bbox), `features.select` / selection sets, `features.add/update/
  delete`, `field.add/rename/delete/calculate` (field calculator),
  `layer.stats` (count/min/max/mean/median/stddev/unique/histogram),
  `layer.filter` (persistent definition query), `layer.join` (attribute join
  by key from another layer/CSV).
- App: **Attribute table (F6)** with sort, filter bar, selection synced with
  canvas, inline edit, field calculator dialog; **Identify panel**.

## Phase 3 - Vector geoprocessing toolbox ~2.5 days

All via `processing.run {algorithm, params}`; each algorithm is a registry
entry with its own spec, test and auto-generated dialog.

- Geometry: buffer (all geometry types, meters, dissolve option), simplify
  (DP / VW), convex hull, centroids, point on surface, bounding boxes, dissolve
  (by field), multipart <-> singlepart, densify, smooth, polygons <-> lines,
  extract vertices, fix/validate geometries, add geometry attributes.
- Overlay: clip, intersection, difference, union, symmetric difference
  (`geo` BooleanOps), split lines by lines.
- Spatial relations: select by location, join attributes by location
  (intersects/within/contains/nearest, with aggregates), count points in
  polygon, distance matrix, nearest (existing).
- Analysis: Voronoi + Delaunay (`spade`), k-means + DBSCAN clustering, hex/rect
  binning with aggregates (grid exists), random points in polygons / bbox,
  kernel density (to raster, Phase 5 output), line/polygon measurements.
- MCP surface stays small: `processing.list`, `processing.describe`,
  `processing.run` (+ existing named tools kept as aliases).

## Phase 4 - Styling, labels, cartography ~2 days

- Style model in `layers.style_json`: single symbol (fill/stroke/width/
  opacity/point size+shape), **categorized** (field -> palette), **graduated**
  (equal interval / quantile / Jenks / pretty, color ramps: viridis-family,
  colorbrewer-style ramps written fresh), **rule-based** (expression ->
  symbol), **labels** (field or expression, size, halo, placement, collision
  avoidance), scale-dependent visibility, layer opacity, draw order, groups.
- One shared CPU renderer (`tiny-skia`, anti-aliased) for headless export, web
  preview and tests; egui canvas reads the same style model.
- Output: real **`export.png`** and **`export.svg`** (map frame, title,
  legend, scale bar, north arrow, attribution), PDF via SVG later. Map
  bookmarks/themes. Auto-style for beginners (pick categorized/graduated from
  field type).

## Phase 5 - Raster and terrain ~2.5 days

- Raster layers referencing files: **GeoTIFF/COG** read (`tiff` + GeoKey
  parsing; single-band float DEM + RGB), **MBTiles** as tile source and offline
  cache export.
- Tools: hillshade, slope, aspect, **contours** (marching squares), raster
  calculator (expression over bands), reclassify, clip by extent/polygon,
  sample raster at points, **zonal statistics**, kernel density output,
  elevation **profile** along a line.
- Keyless DEM download for a bbox (Copernicus GLO-30 / SRTM open buckets).

## Phase 6 - Desktop UX for every level ~2.5 days

- **Beginner**: start page with task wizards (map OSM places, import my GPS
  track/spreadsheet/photos, heatmap, printable map), each a guided dialog
  over the same commands; smart defaults; Ctrl+K locator (places, layers,
  tools, commands); in-app handbook (exists) linked per dialog.
- **QGIS users**: toolbars (Navigation, Attributes, Digitizing), box zoom,
  back/forward extents, measure tool (geodesic), **digitizing** (add/move/
  delete vertices, snapping), undo/redo from the command log, project open/
  save-as/recent (`.gcproj` = the SQLite file), layer groups, CRS selector,
  async tile worker (removes pan stutter, TODO gap 2).
- **Power users/agents**: `pipeline.run` (JSON list of steps, generalizes
  mapforge = the model builder), batch mode over many inputs, command history
  exportable as a replayable pipeline, all tools over `mcp --connect`.
- Track B web console: generic `/api/commands` route from the registry so it
  inherits everything; UI stays thin (map, layers, attribute table, toolbox).

## Phase 7 - Parity, scale, release ~1.5 days

- **Oracle loop**: run the same synthetic inputs through `qgis_process`
  (installed QGIS 3.44 LTR, black-box) and giscraft, diff outputs numerically
  (area/length/count tolerances). Results in `plan/` (local), summary in
  `docs/parity-checklist.md`.
- Scale: 100k-feature layers (R*Tree, paging, render culling), benchmarks.
- `egui_kittest` goldens, NSIS installer, MCPB bundle, TOOLS.md generated
  from the registry, CHANGELOG, glama.json refresh.

## Sizing

~17 agent-days total if run in order. Phase 0 is mandatory first; after it,
1 -> 2 -> 3 is the high-value spine (data with attributes, then queries, then
analysis). 4, 5, 6 can be reordered. Each phase lands as its own commits on
`main` with all gates green; no long-lived branches.

## Open decisions

1. Adopt the pure-Rust geo crate stack (recommended) vs keep hand-rolling.
2. MCP shape: generic `processing.run` + list/describe (recommended, keeps
   tool count ~30) vs one MCP tool per algorithm (80+ tools).
3. Track B web: thin registry-driven driver (recommended) vs freeze.
4. Order after Phase 0: 1 -> 2 -> 3 -> 4 -> 6 -> 5 proposed.
