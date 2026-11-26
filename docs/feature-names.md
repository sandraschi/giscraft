# Feature Names & Trademark Audit (RFC-0029)

All user-facing feature names, menus, tooltips, and primary product descriptions in GISCraft use generic descriptive GIS terminology rather than third-party product trademarks.

## Audit Summary (2026-11-21)

- Removed third-party GIS brand names from:
  - `README.md` pitch and clean-room summary
  - `crates/giscraft/src/main.rs` About dialog
  - `crates/giscraft/src/help_view.rs` Help / Get Started / Licensing views
  - `crates/giscraft-mcp/src/main.rs` CLI `about` string
  - `crates/giscraft/Cargo.toml` package description

## Factual Interoperability Statements (Permitted)

- File formats and open standards: GeoJSON, GeoPackage (`.gpkg`), Shapefile (`.shp`), `.qgz` project archive interop, OpenStreetMap Overpass QL, Komoot Photon geocoding, XYZ slippy tiles.
