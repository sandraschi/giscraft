# qgiscraft-mcp

QGIS-class GIS, storytold craft pattern: 1:1 clone for muscle memory,
modern Rust backend, CLI + MCP first.

- Track A (native): `crates/qgiscraft-app` eframe/egui clone - Project/Layer/Vector/Raster/Processing/Layout menu parity, Browser + Layers docks, map canvas, status bar. Zero ports, in-process `qgiscraft_core::dispatch`.
- Track B (web parallel): ports 11253/11254 reserved for sandraschi webapp for users who never learned QGIS. Same `Command` bus over MCP.
- L1 `qgiscraft-core`: geo types, project model, `Command::dispatch`. L2 `qgiscraft-mcp`: CLI-first + MCP stdio 2024-11-05. Store: SQLite projects.

## Quickstart

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo test --workspace
cargo run -p qgiscraft-mcp -- status --json
cargo run -p qgiscraft-mcp -- mcp
cargo run -p qgiscraft-app --
```

See `docs/target-app-parity.md` for QGIS parity scope, `AGENTS.md` for gates.
