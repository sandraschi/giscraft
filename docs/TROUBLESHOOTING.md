# Troubleshooting

## `overpass status 502` / `photon unreachable`
**Cause**: public demo instances rate-limit or flap (observed repeatedly).
**Fix**: wait a minute and retry. For heavy use, self-host and point
`QGISCRAFT_OVERPASS_URL` / `QGISCRAFT_PHOTON_URL` at it (see Configuration).

## Tiles load slowly or not at all
**Cause**: 2 network fetches per frame by design (no UI freeze); cold cache
on first visit to an area.
**Fix**: pan slowly once (cache is on disk, second visit is instant), or set
`QGISCRAFT_TILES_URL` to a local mirror. Async tile worker is a gaps item.

## `link.exe not found` on `cargo build`
**Cause**: MSVC linker missing (Rust on Windows needs it).
**Fix**: install Visual Studio Build Tools with the "Desktop development
with C++" workload, then rebuild.

## App opens empty / layers gone
**Cause**: it opened a different `qgiscraft.db` (CWD-relative default).
**Fix**: check the status bar project name, or set `QGISCRAFT_DB` explicitly.
Each DB file is a separate project.

## `database is locked`
**Cause**: two writers (e.g. app + CLI ingest at the same instant).
**Fix**: SQLite serializes writers; retry the failed write. Keep one writer
at a time for bulk ingests.

## Zoom does nothing
**Cause**: pre-1.2 builds read only the scroll channel, which egui empties
when Ctrl is held.
**Fix**: update past 1.2 (zoom uses `zoom_delta()` + plain-wheel fallback).
