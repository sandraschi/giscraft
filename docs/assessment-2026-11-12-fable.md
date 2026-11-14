# giscraft assessment - 2026-11-12 (Claude Fable 5.2)

Scope: who it is for, use cases, QoL, esthetics, craft-standard conformance, gaps, ideas, and the
"lens" (museum) scenario incl. speech and iPhone. Basis: code and docs read on Goliath on 2026-11-12.
Not done: the app was not launched or screenshotted for this pass, so esthetics are judged from code.

## 1. State in numbers
- ~12.8k lines of Rust in 5 crates (core, store, mcp, web, app), 142 tests, 18 MCP tools, 3 binaries.
- App is one 3,809-line main.rs. MCP is one 2,270-line main.rs.
- Latest release 0.2.1 (pre-release). Local main is 1 commit ahead of origin, README.md is modified
  (and has CRLF), docs/PHASE0-foundations.md is still untracked.
- git log shows 4 commits, the oldest from 2026-11-09. Earlier history is gone from the branch
  (the code from those commits is still there). Tags/releases v0.1.0 to v0.2.1 still exist on GitHub.

## 2. Who it is for
Today: Sandra and her agents. It is an MCP-drivable map canvas with a keyless OSM pipeline, not a QGIS
replacement. Honest audiences, in order:
1. Agents that need a map engine: typed tools, fenced Overpass, a live app they can drive (--connect).
2. People who want an answer map in a minute without QGIS: single exe, instant start, no keys.
3. The craft fleet as a shared geo service (travelcraft already leans on its tiles and geo math).
4. Niche hobby work: doppler.locate, walk-and-photo field notes.
Not for GIS professionals yet: no reprojection, no Shapefile/GeoPackage/KML/GPX, no raster engine,
no attribute table, no symbology dialog, no print layout (export.png returns a typed error).

## 3. Practical use cases that work now
- "Where are the X in district Y": Overpass query to layer, styled per layer, GeoJSON export.
- CSV/GeoJSON of points to a map, buffer, nearest-hub lines, grid, geodesic circles.
- Paste WKT, jump to a place or coordinate, identify features on hover/click.
- Agent recipes through MCP, incl. map.forge specs.
Close but blocked: a shareable map image (no PNG export), GPS tracks (no GPX), anything offline at
country scale (no PBF loader).

## 4. QoL problems
- Tile fetch is synchronous, 2 per frame: stutter when panning cold areas.
- No undo anywhere, no attribute table, no drag and drop of files, no recent projects, effectively one
  implicit project DB.
- Results and errors only appear in the log panel; a failed Overpass call is easy to miss.
- Colors come from a fixed hash palette; only a per-layer hex override exists.
- Labels only for the selected layer; no copy-coordinates.
- Go to place has no history and the offline table is 15 cities.

## 5. Esthetics (from code)
- Dark visuals only: zero light-theme code. The fleet rule (high contrast, usable light mode) is unmet.
- No custom fonts are loaded, so there is no tofu guard; non-Latin layer names may render as boxes.
- Ctrl+wheel is bound to map zoom. UI scale zoom was not verified.
- Canvas chrome (scale bar, north arrow, color dots) is in place and is the best-looking part.

## 6. Conformance to the craft standards
| Requirement | State |
|---|---|
| L1/L2/L3 crate seam, deny lints, typed errors | Met |
| Command bus shared by CLI, MCP, app | Met (18 tools; parity gate in xtask) |
| 6-gate bar (clippy, tests, fmt, stdio probe, lock, changelog) | Met at 0.2.1; tree is dirty today |
| Menu bar standard of 2026-11-11 (File/Edit/View ... Window/Help) | Not met: 12 QGIS-style menus |
| Help suite: Command Palette, Shortcuts, Files and Docs, Licenses, About | Not met: 0 of the first four |
| switch_on_hover, Preferences, Open Recent, Undo/Redo | Not met |
| Light/Dark theme, Ctrl+wheel UI zoom | Dark only; UI zoom unverified |
| egui_kittest snapshots, tofu guard | Not present |
| Zero ports | Deviates by design: giscraft-web (11253/11254) |
| Release: plain exes, launcher zip only as extra | Met in justfile since 2026-11-09; 0.2.1 asset is still a zip |

## 7. Honesty problems still in the repo
- docs/PRD.md and docs/TIERED_PLAN.md (2026-11-11) tick or claim Shapefile, GeoTIFF, MBTiles, SQL
  filtering, spatial joins, Voronoi, CRS reprojection, R-tree indexing, a 3D canvas, an attribute table,
  symbology controls and a "giscraft-cli" binary. docs/gaps.md and docs/TODO.md list most of these as
  open, and the code agrees with gaps.md. The PRD is a wish list written as status.
- geocode.search with live=false still returns Stephansplatz for any query (core lib.rs line 215).
- The app help text advertises an "Offline Vienna Index (Stephansplatz, Karlsplatz, etc.)"; the only
  offline lookup is the 15-city table.
- 28 docs for 12.8k lines of code, with four assessments and three competing plans.

## 8. The identity conflict
README says "clean-room 1:1 clone shell for QGIS muscle memory". The craft menu standard says
File/Edit/View + domain menus + Window/Help. Both cannot be satisfied. Recommendation: craft bookends
(File, Edit, View, Window, Help) with QGIS domain menus in the middle (Layer, Vector, Raster,
Processing, Web). Drop "1:1" from the README.

## 9. Lens: what exists
- giscraft has no lens code at all.
- The lens is travelcraft SightLens. It does not look at the photo. The photo is a string
  (default "laptop-camera-frame.jpg"); identification is a keyword match on a typed hint
  (caravaggio, klimt, bruegel ...) against a hardcoded catalog, with hardcoded confidence 0.96/0.97,
  and no GPS defaults to Stephansdom. Under the "no gaslights" rule this is a fake.
- Speech there: TTS by Windows SAPI through PowerShell, or a wav from speech-mcp (:10909, up today).
  No microphone path; the UI says to type or paste a transcript. STT sidecar :10914 is down.
- Goliath has what a real lens needs: gemma4 12b/26b (vision) in Ollama, speech-mcp, Tailscale.

## 10. Lens: the museum design
Client (GPD or iPhone) is thin: photo + position + voice in, text + audio out. Goliath does the work.
1. Capture: photo of the painting, and if possible a second of the wall label.
2. Narrow: venue from GPS or a picked museum, to that museum's collection.
3. Identify, in order of reliability: (a) read the label (vision model as OCR), (b) nearest-neighbour
   image match against a prebuilt collection pack (image embeddings of that museum's works from
   Wikidata/Commons), (c) open vision-model guess. Output is a ranked candidate list with the evidence
   for each, and "not sure" is a valid result. No hardcoded confidence.
4. Ground: pull facts for the confirmed work (Wikidata statements, Wikipedia article, museum open data
   where keyless).
5. Analyse: the model writes from those facts plus the photo: what is depicted, composition and light,
   technique, iconography, history and provenance, where it sits in the artist's work, what to look for
   up close. Sourced facts and model commentary are marked apart. Lengths: 30 s, 3 min, 10 min.
6. Speak: sentence-streamed TTS to earbuds, push-to-talk follow-up questions with STT, barge-in.
   Language follows the UI locale. Everything lands in the journal with photo and position.
Giscraft's own lens is geometric: position + compass bearing, ray-cast against OSM building polygons,
"that is the Votivkirche, 340 m". Needs a phone compass; it is pure GIS and needs no model.

## 11. iPhone and GPD connection
Yes. Serve a lens endpoint from Goliath on the tailnet (tailscale serve gives HTTPS, which Safari
requires for camera, mic, geolocation and compass). A web page installed to the home screen covers
both the iPhone and the GPD. The iPhone is not on the tailnet yet (the iPad is). Do not put it on
Funnel without auth. Goliath must be awake and the museum needs mobile data or wifi. A native app on
the admiral-pager chassis can follow once the flow is proven.

## 12. Suggested order (AI-assisted days)
- Day 1: truth pass: PRD/TIERED_PLAN vs gaps.md, geocode fake, help text, commit or drop the dirty files.
- Days 2-3: Tier 4: bookend menus, help suite with command palette, light theme, UI zoom, fonts.
- Day 4: async tile worker, PNG export, GPX import.
- Lens day 1: real identify in a shared lens crate: image in, Ollama vision, candidates out.
- Lens day 2: grounding + long-form analysis + streamed TTS.
- Lens day 3: lens server + home-screen web page over tailscale serve (iPhone and GPD).
- Lens days 4-5: microphone STT with follow-ups, first collection pack (KHM), bearing lens in giscraft.

## 13. Further ideas
- "Ask the map" box: local model turns a sentence into command-bus calls; doubles as command palette.
- Snapshot diff: rerun an Overpass query monthly and show what opened or closed.
- Self-contained HTML map export for sharing.
- Photo EXIF import to a point layer; GTFS stops and lines as layers.
- Walking isochrones from an OSM PBF graph, pure Rust, keyless.