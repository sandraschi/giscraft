# Tentative assessment - giscraft-mcp (2026-10-24, post-0.6)

Method: re-read `docs/target-app-parity.md`, `docs/gaps.md`,
`docs/parity-checklist.md`, `AGENTS.md` against the tree as committed
(`4aff153`). This is a self-review by the building agent, not an
independent assfix. Trust the gates (all green, quoted below), not adjectives.

## 1. What is real (proven, not claimed)

- **Command bus + validation**: 9 commands, hostile inputs become typed JSON
  errors, never panics. `clippy -D warnings` clean incl. `deny(panic/todo)`.
- **SQLite project file**: layers + features + visible + counts, 3->7 col
  migration path, temp-DB hermetic tests. The DB file IS the project.
- **Live paths verified by hand, twice**: Photon `Stephansplatz` -> 3 real
  hits; Overpass `amenity=pharmacy` Innere Stadt -> 29 ingested, read back
  via `features.list`, rendered to `docs/map-preview.png`.
- **First map**: points on canvas (cyan/orange, culled), Ctrl-wheel zoom to
  cursor, drag pan, Fit data, F5 refresh, `--smoke` BMP preview.
- **Gates**: 34 tests (6 app / 13 core / 9 mcp / 6 store), fmt, `xtask ci`
  (fmt+clippy+test+layers+parity), stdio probe byte-exact.

## 2. What is stub, thin, or missing

- **Render is points-only.** Ways/polygons come back from Overpass as center
  points. No lines, no fills, no labels, no per-layer colors.
- **No basemap.** Dots float in a black void. Offline PMTiles Austria is the
  single biggest visual gap vs QGIS.
- **No dialogs that matter yet**: Add Vector/Raster/Delimited/PostGIS/WMS,
  Attribute table (F6), Identify, Processing algorithms, Symbology,
  Print Layout - all listed in the parity doc, none implemented.
  Menu items for these open placeholder labels. That is honest for v0.6
  but the parity doc reads like a plan, not a status.
- **No `.github/workflows/ci.yml`.** New-repo gate wants a Windows CI file;
  `just ci` passes locally but nothing enforces it.
- **No `plan/` oracle material yet.** AGENTS.md describes the
  clean-room loop (screenshots under `plan/qgis/`, never committed) but
  no observation has been recorded. The QGIS side of the parity claim is
  still memory, not measured screenshots.

## 3. Docs have drifted from code (fix before they rot)

- `parity-checklist.md` overclaims:
  - "Toolbars: File/Manage Layers/Digitizing/Attributes/Map Navigation"
    checked - there are no toolbars, only the menu bar.
  - "Canvas: pan/zoom/box-zoom/back-forward" checked - box-zoom and
    back/forward extents do not exist.
  - "`commands --json` lists all commands" - no `commands` subcommand
    exists; `status --json` is the real equivalent.
  - "`ui.screenshot` golden PNGs in `cargo test`" checked - what exists
    is the `--smoke` BMP preview, not kittest goldens in `cargo test`.
  - "Overpass builder + Nominatim locator (Phase 0)" unchecked - but the
    Overpass builder and Photon locator ARE implemented. Stale the other way.
- `AGENTS.md` section 2 is stale:
  - L1 "compiles with only `serde` + `serde_json`" - core now also depends
    on `tokio` + `reqwest` + `giscraft-store` (live I/O moved into dispatch).
  - "The MCP layer only dispatches `Command` IDs" - L2 now owns SQLite
    persistence (`layer_add/list/remove`, `ingest_hits`). The seam moved;
    the doc did not.
- **Typo with staying power**: the env var is `QGISCRRAFT_DB` (double R)
  in `giscraft-store`. Rename to `GISCRAFT_DB` NOW, before any script or
  Track B webapp depends on the misspelling. Cheap today, breaking later.

## 4. Risks

- **Overpass public API flaked once today (502, recovered on retry).**
  No retry/backoff in `execute_ql` (single attempt, 60s timeout). For a
  demo this is fine; for daily use it needs retry-with-backoff or a local
  mirror / PBF-first workflow (which is gap #6 anyway).
- **Hand-rolled MCP framing.** Correct and probed, but every new tool is
  hand-written schema + arm + test. Fine at 9 tools; revisit `rmcp` if the
  surface passes ~20.
- **Core does live network I/O.** The original seam (core = pure, L2 = I/O)
  has eroded: Photon/Overpass live in `dispatch`. It keeps call_tool thin
  and tests stay offline (live paths are opt-in), so this is acceptable -
  but AGENTS.md should say so instead of claiming purity.
- **Ports 11253/11254 are claimed but unused.** No `web_sota/` exists;
  Track B is a reservation, not a track. Either scaffold it or release them
  before someone else claims adjacent pairs expecting a neighbor.

## 5. Verdict (tentative)

Direction is right (clone shell, modern core, CLI+MCP first, DB-as-project),
discipline is holding (gates green 7 commits running, no stubs presented as
done in code). The weak spot is the **docs layer**: checklist and AGENTS.md
already describe a rosier repo than the tree. Fix section 3 (30 minutes),
fix the env typo, add the CI file - then the repo says what it is and the
next render increments (lines/polygons, basemap) have a truthful baseline.

Suggested order: env rename -> docs corrections -> CI file -> basemap spike
(PMTiles Austria offline, read-only underlay) -> ways/polygons.

## 6. Cross-check: Gemini assessment `docs/assessment-2026-10-24.md`

Gemini (as "Antigravity Pair Programmer", DOC-ASSESS-GISCRAFT-20261024-V1)
reviewed baseline `d1438ab` (Phase 0.5). Independent read, same day.
Agreements and corrections:

**Agreed (keeping):** seam split (core-ephemeral vs L2-persistence) is the
top architectural question; geometry must generalize beyond points (WKB or
GeoJSON column); equirectangular is city-scale-only, Spherical Mercator
needed before XYZ tiles align; immediate-mode canvas caps around ~1e4
vertices (vello_cpu path stands); roadmap order basemap -> vectors ->
processing is right.

**Corrections to Gemini's text (it is one commit behind):**
- Test count is **34**, not 28 (6 app incl. projection/fit/BMP).
- Zoom clamp is **0.25..64**, not 0.25..8 - the city-bbox tiny-render bug
  was found by looking at the preview and fixed in `4aff153`.
- Store has **no cascade deletes**: FK cascade is off by default in SQLite,
  child wipe is explicit in `remove_layer`. The "cascade deletes" credit
  in its store row is unearned.
- Locator is **Photon (Komoot)**, not "Photon Nominatim". Nominatim was
  never wired.
- Its mermaid has `Bus --> SQLite`: wrong. Core `dispatch` never touches
  the DB; L2 (`main.rs` helpers) owns persistence. That arrow is exactly
  the seam question, and the diagram picks the side the code did not take.
- Core compiles with `serde/serde_json/reqwest/tokio` **plus an unused
  `giscraft-store` dep** - either use it or drop it (leaving it invites
  exactly this kind of misreading).

**Gemini missed what this assessment caught (still open):**
env typo `QGISCRRAFT_DB`, parity-checklist overclaims (toolbars,
box-zoom/back-forward, `commands --json`, kittest goldens; stale Overpass
line), `AGENTS.md` section 2 staleness, no CI workflow file, Track B
ports reserved but unused.

**Open disagreement:** Gemini recommends unifying `dispatch(cmd, store_ctx)`.
Defensible, but it costs core's hermetic tests (every dispatch test would
need a temp DB) and buys one pipeline where two suffice. Current position:
keep the seam (core = validator + live network, L2 = persistence owner),
fix `AGENTS.md` to say that. Revisit if a third transport appears.

Net: two independent reads agree on architecture and roadmap, disagree on
one seam decision, and both agree the docs layer is the weak spot.
Notably, Gemini's own assessment went stale within one commit - the
strongest argument for the checklist/AGENTS corrections in section 3.
