---
name: giscraft-workflow
description: Step-by-step spatial workflows in GisCraft: geocoding, Overpass ingest, buffer/grid analysis, and WKT preview.
---

# GisCraft Spatial Workflows

1. **Inspect current project:** Call `project.status` and `layer.list` to see existing layers, feature counts, and CRS.
2. **Find a location:** Call `geocode.search` with `live: false` for fast offline lookup (or `live: true` when online).
3. **Run spatial analysis:** After user confirmation, run `processing.buffer`, `processing.grid`, or `processing.nearest` to generate derived vector layers.
4. **Preview geometry:** Use `wkt.show` (after confirmation) to visualize custom WKT geometries on the map canvas.
