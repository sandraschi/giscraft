# AGENTS.md - giscraft (storytold craft pattern, adapted)

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

## 2. Architecture and seams (corrected post-0.6; was aspirational)

- L1 `giscraft-core`: geo types, project model, command bus, plus live
  network clients (Photon, Overpass). UI-free and transport-free, but NOT
  I/O-free: `dispatch` performs keyless live lookups when asked.
  Deps: `serde` + `serde_json` + `tokio` + `reqwest` + `giscraft-store`
  (the store dep is currently unused - use it or drop it).
  Tests stay offline: live paths are opt-in (`live=false` default) and
  never exercised in `cargo test`.
- `giscraft-store`: SQLite project store (rusqlite bundled). Layers +
  features + visible + counts, explicit child-delete (FK cascade is OFF
  in SQLite by default). No UI/MCP/network. Env: `GISCRAFT_DB`.
- L2 `giscraft-mcp`: CLI-first binary + MCP `mcp` subcommand over stdio
  (MCP 2024-11-05, hand-rolled line-delimited framing). No `rmcp` dep.
  L2 OWNS persistence: `layer_add/list/remove`, `ingest_hits`,
  `features_list` live here and call the store directly; core `dispatch`
  is the validator + live-network path, not the sole pipeline.
  (Earlier doc claimed L2 "only dispatches Command IDs" - no longer true
  since 0.2, and that is intentional: it keeps core dispatch tests
  hermetic without temp-DB plumbing.)
- L3 `giscraft`: eframe/egui native clone. Owns the project DB file
  directly (open/seed/list/add/remove/visible persist); calls core
  `project`/`project()` math in-process for rendering, never shells out
  to the MCP binary, never uses local HTTP/JSON-RPC for the desktop path.
  `--smoke` runs open/seed/list + BMP preview headless (no window).
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

## 6. Repo hygiene (fleet rules)

- Line endings: LF only, everywhere (enforced by `.gitattributes`; `*.bat` is the
  single CRLF exception for cmd.exe). Set `core.autocrlf false` in this clone.
  Never commit CRLF; mismatches trip commit hooks.
- Single trunk: work lands on `main`. No long-lived agent branches, no leftover
  WIP branches. If a short branch is unavoidable, merge or delete it in the same session.
- No em dash characters in any file (docs, code, commit messages): use a plain
  hyphen or restructure. They have corrupted file writes in the fleet toolchain.
