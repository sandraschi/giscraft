# Onboarding (first 10 minutes)

No accounts, no keys, no money. Everything the app touches is keyless
(Photon, Overpass, OSM tiles). The one optional install is QGIS itself,
for side-by-side comparison.

1. **Build & run** (see [INSTALL.md](../INSTALL.md)): `cargo run -p qgiscraft --`.
   The app creates `qgiscraft.db` next to where you launched it.
2. **Headless proof**: `cargo run -p qgiscraft -- --smoke` prints layers
   and writes a BMP map preview to temp. No window needed.
3. **First live map**: ingest Vienna pharmacies via CLI (`--live --ingest`),
   then Fit data (or F5) in the app. Tiles stream in progressively.
4. **(Optional) Install QGIS LTR** (`winget install -e --id OSGeo.QGIS_LTR`)
   to click-compare: same bbox, same layers via QuickOSM, screenshot both.
   Record mismatches in `plan/qgis/` (local-only) and `docs/ui-parity.md`.

Pitfalls: Overpass 502s mean "wait a minute" (public instance). A blank
canvas means the wrong DB file (check the status bar name). Zoom needs the
pointer over the canvas.
