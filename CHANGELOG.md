# Changelog - qgiscraft-mcp

## 2026-10-24 - Phase 0.1: real project model + Photon live path
- App owns a real `Project` (add/remove/visibility/select, typed log, 50-line cap).
- Store gains `remove_layer` + `default_db_path`/`open_default` (`QGISCRRAFT_DB` override); CLI/MCP->DB wiring deferred to keep tests hermetic.
- Geocode: `geocode.rs` demo default + `photon_search` live (`--live` CLI, `live` MCP param, 12s timeout, fenced parse).
- Gates green: clippy -D warnings, 19 tests, fmt, xtask ci/layers/parity, stdio probe.

## 2026-10-24 - Scaffold
- Pure-Rust ArtCraft workspace created (core/store/mcp/app + xtask).
- QGIS parity target locked: 1:1 clone for muscle memory, no backend cruft.
- Ports 11253/11254 claimed for Track B web parallel (egui native needs zero ports).
- Gates green: clippy -D warnings, 15 tests, fmt, xtask ci/layers/parity, stdio probe initialize/tools-list/tools-call.
