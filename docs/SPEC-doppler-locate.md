# SPEC: doppler.locate — plane-noise geolocation (offline-first)

Status: v1 shipped (offline matcher: parse + forward model + correlation rank + top-N scored layer). No 920s gap negative-evidence filter yet; single-capture JSON in; emission clamped to track ends. See CHANGELOG (plugins round 2) for the gate record.
Source story: colsto "I discovered a sound that leaks your exact location (and it's everywhere)" https://www.youtube.com/watch?v=UytYrhYLOKI — blurry trees/riverbank clip, no GIS metadata, no landmarks, dated Sept 8, continental US. Two inaudible prop-plane Doppler curves 920s apart matched vs ADS-B to ~100m.

## Why this belongs in qgiscraft

- It is pure GIS once audio is reduced to curves: candidate grid search 70k -> 15k -> top hit, sound-travel + terrain correction, scored vs spectrogram, persisted as layers, visual check over OSM tiles. That is `processing.*` + `layer.*` work.
- It reuses what we already do keyless/offline: Photon/Overpass live-opt-in, tile cache, `processing.buffer` great-circle, `fit_view`, temp-DB hermetic tests.

## Non-goals (explicit)

- No audio decoding / FFT / spectrogram peak-picking in core. Caller supplies curves (time/freq pairs) extracted externally (Audacity, Sonic Visualiser, Python).
- No live ADS-B fetch in v1. No new API key, no account. v1 reads local track files already imported as layers. Live `adsb.*` client is a later, separately gated addition.
- No surveillance tooling. Educational OSINT demo, same caveats as colsto: needs exact timestamp, suitable prop traffic, dense receiver coverage. Outside US/EU lowlands ambiguity explodes.

## Proposed surface (1 new Command, hermetic)

`CMD_DOPPLER_LOCATE = "doppler.locate"`

Params (all validated like `overpass.query` bbox):
- `timestamp`: RFC3339 recording time, required. No timestamp = typed error, no search.
- `curves`: array of 1..8 Doppler curves, each `{t0, freqs: [f64]}` or `{t0, f0, slope}` minimal form for v1. Frequencies Hz, times seconds relative to `timestamp`. Empty = typed error.
- `south/west/north/east`: bbox, required all-four or none (none = CONUS default rejected — require explicit bbox in v1 to bound compute).
- `tracks_layer`: layer_id of ingested ADS-B tracks (from `layer.import` CSV/GeoJSON: `time, lng, lat, alt_m, speed_ms, heading`). Missing layer = typed error.
- `live`: bool, default false. v1 ignores live except to return `live:false + note: offline matcher only`.
- `db`, `output_layer`: same conventions as `processing.buffer`.

Result JSON:
`{op:"doppler.locate", count, top:{lng,lat,score}, hits:[{lng,lat,score}], output_layer, live:false}` — hits persisted as point layer via L2 store, same as `overpass --ingest`.

## Math module (new `crates/qgiscraft-core/src/doppler.rs`)

Pure, no network, no UI, no store:
- `doppler_freq(f0, vsrc_radial, c=343.0) -> Result<f64>` with `checked_*` / finite checks.
- `predicted_curve(candidate:{lng,lat}, track samples, terrain_m) -> Result<Vec<f64>>` — includes sound-travel delay iteration (2 passes max, bounded), altitude MSL minus terrain for 3D range.
- `score_curve(observed, predicted) -> Result<f64>` — normalized correlation, no panics on len mismatch (typed error).
- Grid search helper `rank_candidates(bbox, step_deg, curves, tracks) -> Result<Vec<Scored>>` with `checked_mul` on grid dims, hard cap (e.g. 100k cells) returning typed error if exceeded.

All fallible -> `Result<T, GisError>`, no indexing (`get()`), no byte slicing, lng/lat validated before math. Terrain in v1: single `terrain_m` param per call (SRTM lookup is later).

## Tests (must pass before merge)

- Forward model: straight-line constant-alt flyover produces falling-then-rising? Actually falling pitch crossing — assert monotonic drop through closest approach on synthetic track.
- Scoring: identical curves score ~1.0, time-shifted 920s-apart pair resolves to correct cell in 10x10 synthetic grid.
- Hostile inputs: empty curves, NaN freq, zero-step bbox, 10M-cell grid request, missing tracks layer — all typed errors, zero panics.
- Round-trip: `DopplerCurve` serde JSON round-trip.
- `dispatch(Command::DopplerLocate{...})` offline returns `live:false` + top hit matching direct `doppler::rank_candidates` call (parity).

## L2 / L3 wiring

- L2 `qgiscraft-mcp`: `doppler-locate` CLI + `doppler.locate` tool (12th tool). `tools/list` description: short, imperative, units (degrees, Hz, seconds, meters). Forwards to `dispatch`, then persists hits via store `replace_features` like `overpass --ingest`. Stdio framing unchanged.
- L3 app: no new dock in v1. Result layer renders as colored dots (existing palette + white halo) + auto `fit_view` + `processing.buffer` 100m ring for the top hit. A "Doppler import" menu entry can wait for v2.

## Privacy + clean-room notes

- Ship with colsto's caveats in `docs/`: high-pass / noise / phone compression do NOT remove the signal; cutting / extra NR makes it harder; author says 100 easier ways to find you. Mark as OSINT research, not a tracker.
- Reference impl `github.com/digitalis-p/doppler-locate` is for ideas only — reimplement forward model from Doppler physics, don't copy code. Same rule as QGIS bundle.

## Acceptance (6-gate)

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
2. `cargo test --workspace` green (87 baseline + new doppler tests).
3. `cargo fmt --all --check` clean.
4. `tools/list` + `tools/call` stdio probe with new tool present.
5. No `*.bak`, `Cargo.lock` committed, no `target/` in status.
6. `CHANGELOG.md` line: `doppler.locate offline matcher (curves + tracks layer -> scored layer)`.

## Beauty/usefulness tie-in (same love pass)

- Remove stale `*.bak` litter (`main_*.bak`, `lib_*.bak`, `TOOLS_*.bak`) — they are ignored but ugly.
- `.gitignore`: add `*.bak.*` (fleet gate requires both `*.bak` and `*.bak.*`).
- `main.rs` 114KB single file: split into modules (`views/`, `dialogs/`) in a follow-up, not in this SPEC — too big for one commit (10-file rule).
- Usefulness order stays: GPX import (Gap 3) unblocks field tracks for doppler v2, async tiles (Gap 2) unblocks smooth candidate inspection, `export.png` (Gap 7) unblocks shareable result maps.
