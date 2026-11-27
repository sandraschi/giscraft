# Troubleshooting

## `overpass status 502` / `photon unreachable`
**Cause**: public demo instances rate-limit or flap (observed repeatedly).
**Fix**: wait a minute and retry. For heavy use, self-host and point
`GISCRAFT_OVERPASS_URL` / `GISCRAFT_PHOTON_URL` at it (see Configuration).

## Tiles load slowly or not at all
**Cause**: missing tiles are decoded on a background worker. A cold area
still waits on the network. A failed tile is tried again after 2 seconds.
The one-shot BMP preview (`--smoke`) fetches its own tiles and is not that worker.
**Fix**: visit the area once (the cache is on disk, the next visit is instant),
or set `GISCRAFT_TILES_URL` to a local mirror.

## `link.exe not found` on `cargo build`
**Cause**: MSVC linker missing (Rust on Windows needs it).
**Fix**: install Visual Studio Build Tools with the "Desktop development
with C++" workload, then rebuild.

## App opens empty / layers gone
**Cause**: it opened a different `giscraft.db` (CWD-relative default).
**Fix**: check the status bar project name, or set `GISCRAFT_DB` explicitly.
Each DB file is a separate project.

## `database is locked`
**Cause**: two writers (e.g. app + CLI ingest at the same instant).
**Fix**: SQLite serializes writers; retry the failed write. Keep one writer
at a time for bulk ingests.

## Zoom does nothing
**Cause**: pre-1.2 builds read only the scroll channel, which egui empties
when Ctrl is held.
**Fix**: update past 1.2 (zoom uses `zoom_delta()` + plain-wheel fallback).
