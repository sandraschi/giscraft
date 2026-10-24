# AGENTS.md - qgiscraft-mcp (storytold craft pattern, adapted)

QGIS-class GIS: clean-room 1:1 clone for muscle memory, modern Rust backend,
CLI + MCP first. Dual track: Track A native egui clone, Track B sandraschi
webapp parallel on 11253/11254 for users who never learned QGIS.

## 1. Safety and error handling (never crash)

- Workspace lint gate (root `Cargo.toml`):
  `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`,
  `clippy::unimplemented`, `clippy::todo` are all `deny`.
- Every fallible function returns `Result<T, GisError>` and bubbles with `?`.
  Use `ok_or_else` / explicit `match`, never `unwrap` / `expect`.
- Treat all external input as hostile (CLI args, MCP payloads, GeoJSON, WKT,
  Overpass QL, file bytes):
  - No direct indexing (`buf[i]`); use `.get()` with a typed error on `None`.
  - Integer math on lengths / counts / zoom uses `checked_*` / `saturating_*`.
  - String slicing only at `char` boundaries (`str::get`, never byte slicing).
  - Validate `lng -180..180`, `lat -90..90`, zoom `0..24` before any geo math.
- Lock poisoning must never cascade: `lock().unwrap_or_else(PoisonError::into_inner)`
  pattern or restructure so workers report error events instead of killing the server.
- Every bug fix MUST ship a minimal regression test that fails before the fix.
- Stdio MCP invariant: **stdout is pure JSON-RPC only**. All logs go to stderr
  via `eprintln!`. Audit every crate for rogue `println!` outside the framed writer.

## 2. Architecture and seams

- L1 `qgiscraft-core`: geo types, project model, command bus. Zero UI,
  zero MCP, zero eframe dependencies. Compiles with only `serde` + `serde_json`.
- `qgiscraft-store`: SQLite project store (rusqlite bundled). No UI/MCP.
- L2 `qgiscraft-mcp`: CLI-first binary + MCP `mcp` subcommand over stdio
  (MCP 2024-11-05, hand-rolled line-delimited framing). No `rmcp` dep in v0.1.
  The MCP layer only dispatches `Command` IDs into `qgiscraft-core`.
- L3 `qgiscraft-app`: eframe/egui native clone. Calls `qgiscraft-core`
  functions directly in-process. Never shells out to the MCP binary,
  never uses local HTTP/JSON-RPC for the desktop path.
- `xtask`: `ci` (fmt+clippy+test+layers+parity), `layers` (L6 UI swappable check),
  `parity` (presence checklist from `docs/parity-checklist.md`).
- No agent may invent ad-hoc state transitions between UI widgets and core buffers.
  New capability = new `Command` variant + typed params + dispatch-loop unit test.

## 3. Clean-room (storytold rule)

- QGIS is installed and may be *observed* black-box: run it, use its UI with
  synthetic documents, screenshots under `plan/qgis/` (never committed).
- Never read, disassemble or copy anything inside the QGIS bundle beyond
  feature names, never copy icons, artwork, presets or wording beyond
  feature names, never commit files produced by QGIS. Never copy GPL code.
- `plan/` is gitignored (local only). Parity docs in `docs/` are committed.

## 4. MCP surface contract

- Every command exposed via MCP MUST have:
  1. A unique string command identifier (`project.status`, `layer.add`, ...).
  2. Strongly typed deserializable params with JSON Schema in `tools/list`.
  3. A headless dispatch unit test (no network, no hardware).
- `tools/list` descriptions are the agent contract: keep them short, imperative,
  with units (degrees, meters, zoom) and arg ranges.
- Notifications (`notifications/initialized`) get no response. Unknown methods
  return JSON-RPC `-32601`, bad params `-32602`, parse failures `-32700`.

## 5. Definition of done (6-gate)

- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` green.
- [ ] `cargo test --workspace` green (incl. new regression test for any fix).
- [ ] `cargo fmt --all --check` green.
- [ ] Round-trip serialization test for any new schema.
- [ ] `tools/list` + `tools/call` smoke-tested over stdio with byte-exact framing.
- [ ] CHANGELOG.md checkpoint line added.
