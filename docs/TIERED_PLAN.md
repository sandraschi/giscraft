# GISCraft - Tiered Execution Plan

**Refreshed:** 2026-11-14 (Claude Fable 5.2), replacing the 2026-11-11 version whose Tier 1-3 boxes
were ticked for things the code does not do. Rule for this file: a box is ticked only when the code
does it today and a test or gate proves it. Open work lives in `docs/gaps.md` and `docs/TODO.md`.
**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)
**Assessment behind this plan:** [`assessment-2026-11-12-fable.md`](assessment-2026-11-12-fable.md)

## State on 2026-11-14
- Rust sources are unchanged since 2026-11-08 (release 0.2.1). Everything since is docs and build
  scaffold: `scripts/release.ps1`, `installer/craft-installer.nsi`, `mcpb/manifest.json`, `start.ps1`.
- The new release scaffold has not cut a release yet. 0.2.1 on GitHub is still a zip.

## Tier 0 - Truth and hygiene (done 2026-11-14)
- [x] `docs/PRD.md`: sections 1 and 2 now separate "works today" from goals.
- [x] `geocode.search` with `live=false` answers from the offline city table and returns an
      `offline` error for anything else. `demo_search` is gone; travelcraft `travel_geocode` uses
      the same `offline_search`.
- [x] App help text no longer claims an "Offline Vienna Index".
- [x] Dirty files committed, README is LF.
- [x] Old assessments moved to `docs/archive/`; docs index points at this plan.
- [ ] Still to fold: `PLAN-power-gis.md` and `TODO.md` overlap with `gaps.md`.
## Tier 1 - Core engine (`giscraft-core`, `giscraft-store`)
- [x] `Command::dispatch` bus; store-backed ops return a typed error in the stateless core.
- [x] Points, lines, polygons; WKT and GeoJSON parse; ear-clip triangulation.
- [x] Web Mercator projection for display, shared by app, headless preview and web.
- [x] Deny lints: unwrap, expect, panic, todo, unimplemented. Typed `GisError` everywhere.
- [x] Fenced Overpass builder with retries; Photon geocoding; offline city table (15 cities).
- [x] Processing: buffer (points), grid, nearest hub, geodesic shapes, doppler.locate MVP.
- [ ] `#![forbid(unsafe_code)]` in every crate (not present today; 0.5 h).
- [ ] Spatial index (R-tree). Not present; bbox filters are linear scans.
- [ ] Layer reprojection (CRS to CRS). Not present.
- [ ] GPX and KML import (1 day). Then xlsx, EXIF photos, MBTiles.
- [ ] Clip, dissolve, buffer for lines and polygons (1-2 days).
- [ ] OSM PBF bulk loader for offline country extracts (2 days).

## Tier 2 - Headless driver (`giscraft-mcp`)
- [x] One binary: CLI subcommands with `--json`, and `mcp` for the stdio server (18 tools).
      There is no `giscraft-cli` binary and no FastMCP; the stdio loop is hand-rolled.
- [x] `--db` persistence; `--connect PORT` forwards to the live app.
- [x] Errors set `isError`; CLI exits 1 on error; import returns a summary.
- [x] GeoJSON and CSV import, GeoJSON export.
- [ ] `export.png` (returns a typed error today). Print layout with legend, scale bar, title (1 day).
- [ ] MCP twins for every processing dialog; xtask parity already guards the list.
- [ ] `--version` flag on the binary.

## Tier 3 - Desktop app (`giscraft`)
- [x] egui canvas: pan, wheel/Ctrl+wheel/pinch map zoom, tile underlay, scale bar, north arrow.
- [x] Layer dock with visibility and color; identify on hover and click; Go to place (Ctrl+G).
- [x] File pickers for import/export; processing toolbox dock; 7-locale menu strings.
- [x] Async tile worker (the frame uploads decoded textures; fetch runs off the UI thread).
- [x] Attribute table dock (F6) (name, kind, coordinates; row click highlights the feature).
- [ ] Symbology: point size, stroke, opacity, categorized and graduated (1-2 days).
- [ ] Undo/redo for layer and feature edits (1 day).
- [ ] Drag and drop files; recent projects; project files instead of one implicit DB (1 day).
- [ ] Errors surfaced as a toast or banner, not only the log panel (0.5 day).
- [ ] Split the 3,809-line `main.rs` into modules (0.5 day, before the items above).

## Tier 4 - Craft menu bar, help suite, look, packaging (2-3 days)
- [ ] Bookend menus: File, Edit, View, then GIS domain menus (Layer, Vector, Raster, Processing,
      Web), then Window, Help. README drops the "1:1 QGIS clone" promise.
- [ ] Help suite: Command Palette (Ctrl+K), Keyboard Shortcuts (F1), Project Files and Docs,
      Licenses and Notices, About.
- [ ] `switch_on_hover`, Preferences, Open Recent.
- [ ] Light theme with high contrast (dark only today) and Ctrl+wheel UI zoom outside the canvas.
- [ ] Bundled fonts with CJK coverage and a tofu guard test.
- [ ] `egui_kittest` snapshot tests.
- [ ] First release cut through `scripts/release.ps1`: plain exes, launcher zip as an extra only,
      NSIS installer, mcpb. Scaffold exists since 2026-11-12, unproven.

## Tier 5 - Lens (shared engine, decided 2026-11-14)
The lens engine is a shared crate set in its own repo (working name `lenscraft`), used by travelcraft
(owner of the SightLens tab and journal), by giscraft (venue lookup, bearing lens) and by a small
server for phone and GPD. Today's travelcraft SightLens matches a typed hint against a hardcoded
catalog and never opens the photo; it gets replaced, not extended.

| Step | Work | Days |
|---|---|---|
| L1 | DONE 2026-11-14: `D:\Dev\repos\lenscraft` (`lenscraft-core` + `lenscraft-mcp`), photo in, ranked candidates with evidence out, verdict + basis, 27 tests, live-tested | 1 |
| L2 | DONE 2026-11-22: label-first mode, Overpass venue narrowing, Wikidata/Wikipedia grounding, 30s/3m/10m analysis with sourced fact verification | 1 |
| L3 | DONE 2026-11-22: sentence-streamed TTS via `speech-mcp` (`:10909`) | 0.5-1 |
| L4 | DONE 2026-11-22: `lens-server` (port `11265`) + mobile web UI + `scripts/tailscale-serve.ps1` | 1 |
| L5 | DONE 2026-11-22: push-to-talk STT (`speech-mcp`), grounded follow-up Q&A (`qa.rs`), web UI barge-in | 1 |
| L6 | PARTIAL 2026-11-22: `travelcraft-core/src/lens.rs` updated for L2 grounding/explain + L5 Q&A; egui tab UI polish open | 0.5 |
| L7 | DONE 2026-11-22: `giscraft-core/src/bearing_lens.rs` position + heading ray-cast against OSM building footprints | 1 |
| L8 | DONE 2026-11-22: `lenscraft-core/src/collection.rs` Wikidata SPARQL pack builder (KHM `Q95569` default) + 24-dim spatial color signature matcher | 2 |

Lens is usable in a museum after L4 (about 4 days). L5 to L8 raise quality.

L1 findings that shape L2 to L8 (live, gemma4:12b): about 3 s per photo warm; recognition alone
gave a confidently wrong title about one run in five, so `identified` needs text in the photo
that names the title, and L2 grounding (Wikidata) is the next lever. Text-dense photos make the
model loop; capped and retried once. Ollama reports `audio` capability for gemma4:12b and e4b,
so speech in (L5) may not need a separate STT engine: test before choosing speech-mcp or speechcraft.

## Order
1. Tier 0 (0.5 day).
2. Lens L1 to L4 (4 days), in parallel with nothing else in giscraft sources.
3. Tier 4 (2-3 days), starting with the `main.rs` split.
4. Tier 3 QoL still open: error toasts, undo, drag-and-drop, recent projects.
   Async tiles and the F6 attribute table landed 2026-11-17.
5. Tier 2 PNG export, Tier 1 GPX.
6. Lens L5 to L8.