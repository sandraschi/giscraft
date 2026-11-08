# UI and map parity vs QGIS 3.44.15-Solothurn (LTR)

> **Last reviewed:** 2026-10-24 · **Target:** QGIS 3.44.15 LTR, fresh profile,
> Windows · **Oracle install:** `C:\Program Files\QGIS 3.44.15\bin\qgis-ltr.bat`
> Observations live in `plan/qgis/` (local-only, never committed).

Black-box rule: every row is measured against the running QGIS, never from
memory or docs. Unverified rows are marked TODO, not assumed.

**Dimension score: not yet measured (target: first 10 rows by 0.2).**

## Measured

| Area | QGIS 3.44 | giscraft | Status | Evidence |
|---|---|---|---|---|
| Canvas wheel zoom (no modifier) | TODO observe | implemented (plain-wheel path) | ~ | needs side-by-side check |
| Canvas Ctrl+wheel zoom | TODO observe | implemented (zoom_delta path) | ~ | was broken pre-1.2 (smooth-channel only); verify live |
| Canvas drag pan | TODO observe | implemented (grab-and-drag) | ~ | verify direction matches |
| Box zoom / back-forward extents | TODO observe | missing | ✗ | gaps |
| Layers dock (checkbox/visibility/count) | TODO observe | working prototype | ~ | smoke + screenshots |
| Status bar CRS/scale/coords | TODO observe | project file/z/scale/status | ~ | labels differ deliberately (DB file vs .qgz) |
| Menu bar parity | TODO inventory | Project/Layer/View/Help wired, rest stubbed | ~ | `plan/qgis/02-ui-ux.md` |
| Processing Toolbox / Symbology / Layout | TODO observe | stubbed | ✗ | gaps |

## Revision history

| Date | Change |
|---|---|
| 2026-10-24 | Created: oracle installed (3.44.15 LTR), scaffold with first rows. Nothing measured yet - honest zero. |
