# Changelog - qgiscraft-mcp

## 2026-10-24 - Scaffold
- Pure-Rust ArtCraft workspace created (core/store/mcp/app + xtask).
- QGIS parity target locked: 1:1 clone for muscle memory, no backend cruft.
- Ports 11253/11254 claimed for Track B web parallel (egui native needs zero ports).
- Gates green: clippy -D warnings, 15 tests, fmt, xtask ci/layers/parity, stdio probe initialize/tools-list/tools-call.
