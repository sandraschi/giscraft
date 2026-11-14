# Phase 0 foundations: design notes and handoff

Status: designed and implemented in a separate Claude session on 2026-11-08,
**not merged**. A parallel session is working on the repo, so this file hands
the ideas over instead of the code. Companion to `docs/PLAN-power-gis.md`
(Phase 0 section). A full patch exists (`giscraft-phase0.patch`, against
`main` @ 23e640b, 25 files, +4317/-573) in Sandra's downloads if wanted;
expect conflicts once `main` moves, so treat it as reference, not as a merge.

Verified in that session (Linux, Rust 1.97): fmt, clippy -D warnings,
163 tests (was 133), `xtask ci`, end-to-end CLI + MCP stdio runs. Not yet
verified on Goliath (Windows, Rust 1.99, real `giscraft.db` migration).

Decisions Sandra made (2026-11-08): adopt the pure-Rust geo crate stack;
MCP shape = generic `processing.run` + `processing.list/describe` for the
Phase 3 toolbox (named tools stay as aliases); phase order after 0 is
1 I/O, 2 attributes, 3 toolbox, 4 styling, 6 UX, 5 raster.

## Why Phase 0 first

A feature was `name, kind, lng, lat, geom_json`. **No attributes**: GeoJSON
properties, CSV columns and OSM tags were dropped on import, so attribute
table, filters, categorized styling, field calculator and joins were all
impossible. No spatial index, no reprojection, three copies of the ingest
code (CLI/MCP, web sidecar, app), and 18 hand-written MCP schemas duplicated
by clap args and app dialogs.

## 0a Store v2 (`giscraft-store`)

- `StoredFeature` gains `fid: i64` (stable row id) and
  `props: serde_json::Map<String, Value>`; derive `Default` so the ~27 struct
  literals across crates take `..Default::default()`.
- Migration via the existing ALTER-and-ignore-duplicate pattern:
  - `features.props_json TEXT NOT NULL DEFAULT '{}'`
  - `features.bminx/bminy/bmaxx/bmaxy REAL` (exact f64 bbox, see pitfall 1)
  - `layers.style_json, filter_expr, opacity REAL DEFAULT 1.0,
    sort_order INTEGER, grp, source_crs` (all TEXT defaults '')
  - new `layer_fields(layer_id, ord, name, ftype)`, PK (layer_id, name)
  - new `CREATE VIRTUAL TABLE features_rtree USING rtree(id, minx, maxx, miny, maxy)`
    (bundled SQLite in rusqlite has the R*Tree module; no new dep)
  - new `meta(key, value)` with `schema_version`; a one-time backfill fills
    rtree + bbox columns for pre-v2 rows, idempotent.
- `replace_features` runs in one transaction (`conn.unchecked_transaction()`
  works on `&Connection`), prepared statements, writes rtree rows and bbox
  columns, then infers and writes the field schema.
- Field inference: first-seen order; null skipped; int+real -> real; any
  other mix -> text. Types: int | real | text | bool.
- New API: `query_features(FeatureQuery{layer_id, bbox, limit, offset})`,
  `all_features`, `count_features(layer, bbox)`, `layer_extent`,
  `layer_fields`, `set_fields`, `layer_meta` / `set_layer_meta`,
  `geom_extent(geojson_str)`, `infer_fields`. `list_features` stays (500 cap)
  as a wrapper. `remove_layer` / `clear_all` also delete rtree rows and fields.
- Tests: props/fields/bbox/extent/paging, exact edge (1e-7 deg outside must
  not match), meta roundtrip + opacity fence, and a test that builds the
  exact pre-v2 schema, inserts rows, runs `init_db` twice and checks backfill.

## 0b Geometry engine and attribute-preserving import (`giscraft-core`)

- Workspace deps: `geo 0.33`, `geo-types 0.7`, `geojson 1.0`,
  `wkt 0.14` (default-features off, `geo-types` on). All MIT/Apache.
- New `core::geom` bridge (the only place text becomes geometry):
  `from_geojson_str/from_value/to_value/to_geojson_string`, `of_feature`
  (geom_json or Point at lng/lat), `validate_lonlat` (finite, in range,
  vertex cap 2M, error says "reproject first"), `representative_point`
  (`InteriorPoint` first, so labels sit inside concave polygons; centroid
  fallback), `class_of` / `layer_class` (point/line/polygon/mixed,
  dominant mode for mixed layers), `bbox`.
- `RawFeature` gains `props` + `Default`, plus `from_geometry(name, &g, props)`
  and `to_hit()` (the `{name, kind, lng, lat, geom, props}` hit shape).
- GeoJSON import via the `geojson` crate: FeatureCollection, Feature or bare
  geometry; every RFC 7946 type incl. Multi* and GeometryCollection;
  properties kept; null geometry is a typed error with the index; cap 200k.
  Name picked from props by `name|title|label|id|fid|ref` (case-insensitive).
- CSV import rewritten: RFC 4180 quoting incl. quoted newlines, BOM strip,
  delimiter sniff (tab, semicolon, comma), decimal commas in coordinates,
  every non-geometry column kept as a typed attribute (`typed_cell`: int,
  real, bool, else text; leading-zero codes like postcodes stay text; empty
  = null; "NaN" stays text). Geometry from a WKT column
  (`wkt|geometry|geom|the_geom|wkt_geom`, EWKT `SRID=n;` prefix tolerated)
  or lng/lat columns.
- Overpass: `OverpassHit.props` = all string tags (cap 256) + `osm_type`,
  `osm_id`.
- New `core::persist`: one `ingest_hits(db, layer_id, name, source,
  geom_type, hits, op)` + `ingest_hits_conn` + `hits_to_features` +
  `infer_geom_type`. The MCP and web copies become one-line delegates; the
  app's RawFeature<->StoredFeature conversions carry `props`.

## 0c Command registry (`core::registry`)

- `CommandSpec { id, group, summary, params: &[ParamSpec], mutates }`,
  `ParamSpec { name, ty (Str|Num|Int|Bool|Obj|Arr), required, desc, unit,
  min, max, default (JSON text), choices }` with const builder methods.
- One static `COMMANDS` table generates: MCP `tools/list` (JSON Schema with
  description incl. unit, minimum/maximum, default, enum), `commands --json`,
  `describe <id>`, and `docs/TOOLS.md` (`cargo run -p xtask -- tools-md`;
  `xtask ci` fails when TOOLS.md drifts; xtask parity iterates the registry
  instead of a hard-coded list).
- `spec.validate(args)` runs in front of every MCP `tools/call` and CLI
  call: unknown params, missing required, wrong JSON type, out of range,
  bad choice -> typed `invalid input` error. MCP results with an error get
  `isError: true`. Three existing MCP tests needed new expected messages.
- CLI: `commands [--json]`, `describe <id>`, generic
  `call <tool> --args '{...}' [--connect PORT]` (same path as MCP), plus
  `layer-info`, `features-query`, `layer-export`; `project-status --db`.
  The duplicated CLI `layer-import` body routes through the MCP handler.
- Core `status` lists `registry::ids()`.

### New store-backed ops (`core::ops`, JSON in/out, take `&Connection`)

- `project.status`: db path, schema version, layer/visible/feature counts,
  overall extent.
- `layer.info`: fields, extent, count, CRS, style/filter/opacity/group,
  `source_crs`.
- `features.query`: `layer_id` + optional bbox, `limit` 1..1000 (default
  100), `offset`, `fields` subset, `geometry` flag; returns `total` and
  `next_offset` (null on the last page).
- `layer.export`: GeoJSON (all props, `id` = fid, `name` added only if no
  name-like prop) or CSV (schema-order columns; lon/lat for all-point
  layers, else a WKT column via a new `wkt::to_wkt` writer). Both formats
  round-trip through our own importers (tested).

### Honesty fixes found on the way

- `project.status` over CLI/MCP returned a hard-coded
  `"wien-demo", layers: 0`. Core `ProjectStatus` / `LayerList` arms returned
  fake empty data. Fix: real op; core arms return typed `unsupported`.
- `layer.import` echoed every imported row in its result (a 200k-row file
  floods an agent). Fix: return `layer.info` summary + `ingested`.

## 0d CRS (`core::crs`)

- `proj4rs 0.1` with features `crs-definitions` + `aeqd`, plus
  `crs-definitions 0.4` (CC0; WKT included for names, costs a few MB of
  binary; also useful for Phase 1 `.prj` matching).
- `resolve(def)`: `EPSG:n`, `n`, `urn:ogc:def:crs:EPSG::n`, `WGS84`,
  `CRS84`, or a `+proj` string. `epsg_name(code)` from the WKT,
  `info(def)`, `search(query, limit)` (all words must match, scans the
  u16 code space), `transform_xy` (handles radians: proj4rs uses radians
  for geographic CRSs in and out), `transform_geometry` (geo
  `try_map_coords`), `to_wgs84`, `local_aeqd(lng, lat)` for metric
  processing in Phase 3.
- **Datum shift table** (pitfall 3): append `+towgs84` for MGI
  (31251-31259, 31281-31288): `577.326,90.129,463.919,5.137,1.474,5.297,2.4232`;
  DHDN (31461-31469, 31491-31495): `598.1,73.7,418.2,0.202,0.045,-2.455,6.7`;
  CH1903 21781: `674.4,15.1,405.3,0,0,0,0`; CH1903+ 2056:
  `674.374,15.056,405.346,0,0,0,0`; OSGB36 27700:
  `446.448,-125.157,542.06,0.15,0.247,0.842,-20.489`; Amersfoort 28992:
  `565.417,50.3319,465.552,-0.398957,0.343988,-1.8774,4.0725`.
  Metre-level, no NTv2 grids; say so in docs.
- Import: `layer.import` takes `crs`; GeoJSON honours the legacy `crs`
  member when no explicit CRS is given; projected CSVs look for
  x/easting/rechtswert and y/northing/hochwert columns; source CRS is stored
  in layer meta. Import validation errors suggest `crs=EPSG:<code>`.
- Tools: `crs.info`, `crs.search` (limit 1..200), `crs.transform`
  (`from`, `to` default EPSG:4326, `coordinates` [[x,y],...] max 100k,
  and/or a GeoJSON `geometry`).
- Tests: Web Mercator exact to 1 cm (x = R*lon, y = R*ln(tan(pi/4+lat/2))),
  UTM 33N / MGI GK East / LAEA round trips to 1e-7 deg, OS projection worked
  example exact to 1e-7 deg on the Airy ellipsoid, OSGB36 Helmert shift moves
  the point 100-200 m westward, MGI shift > 50 m, ETRS89 codes untouched,
  aeqd distance (0.1 deg lon at 48N ~ 7455 m), search finds EPSG:31256.

## Pitfalls hit (save the next session the time)

1. **SQLite R*Tree stores f32** and rounds outward. Fine as a prefilter,
   wrong for reported extents (16.3606 came back as 16.360599517822266) and
   bbox edges. Keep exact f64 bbox columns on `features` and apply them after
   the rtree subquery; compute extents from those columns.
2. **`serde_json::Map` sorts keys** unless the `preserve_order` feature is
   on. Attribute tables must keep source column order: enable
   `serde_json = { version = "1.0", features = ["preserve_order"] }`
   workspace-wide (two tests assumed alphabetical order and needed fixing).
3. **crs-definitions omits `+towgs84`** for classic national datums (MGI,
   DHDN, CH1903, OSGB36, Amersfoort): Austrian data lands ~100-150 m off
   without the table above. Modern GRS80/ETRS89 and WGS84 UTM codes are fine.
4. The OS "Caister water tower" coordinates are the OSGB36 lat/lon of the
   *projection* example, not ETRS89; use them to test tmerc without a shift.
5. The hand-rolled `wkt.rs` parser range-checks every vertex as lon/lat, so
   projected WKT fails before reprojection; use the `wkt` crate for CSV WKT
   columns and keep `wkt.rs` for `wkt.show`.
6. Struct-literal fixups by script: skip `-> Type {` (function return types)
   and do not add `..Default::default()` to complete literals (clippy
   `needless_update`).
7. Clippy name clash: a clap variant `Commands::Commands` trips
   `enum_variant_names`; name it `CommandList` with `#[command(name = "commands")]`.
8. `cargo test` / `clippy` do not refresh `target/debug/giscraft-mcp`;
   `cargo build` before CLI smoke runs.

## Deferred

- Splitting `crates/giscraft-mcp/src/main.rs` (80 KB) into modules: do it in
  Phase 1 while moving handlers onto the registry.
- App and web do not yet use `core::ops` for the new tools (the app control
  arm forwards only the old set under `--connect`).
