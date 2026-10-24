# Development Setup

Pure-Rust ArtCraft workspace (see `RUST_ARTCRAFT_STANDARD.md` in
`mcp-central-docs/standards/`). No Python, no Node, no Docker.

## Tools Required

```powershell
winget install Rustlang.Rustup
# + Visual Studio Build Tools, "Desktop development with C++" (MSVC linker)
winget install Git.Git
# just is optional but recommended (justfile recipes below)
```

Verify: `cargo --version` (1.85+; workspace pins `Cargo.lock`).

## Setup

```powershell
git clone https://github.com/sandraschi/qgiscraft-mcp
cd qgiscraft-mcp
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo test --workspace   # 56 tests, all offline
```

## Common Tasks (`justfile`)

| Recipe | What |
|---|---|
| `just check` | clippy workspace, `-D warnings` (the gate) |
| `just test` | `cargo test --workspace` |
| `just fmt-check` / `cargo fmt` | formatting gate / apply |
| `just ci` / `just parity` / `just layers` | full gate / parity checklist / layering via `xtask` |
| `just run-mcp` / `just run-cli` | MCP stdio server / CLI smoke |
| `just run-app` / `just run-app-smoke` | egui app / headless smoke + BMP preview |
| `just build-release` | production build |

## Code Standards

- `AGENTS.md` in the repo root is normative (gates, seams, clean-room rules).
- Workspace lints deny `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented` — every failure is a typed value, every test hermetic
  (unique temp DBs, no network in `cargo test`, no env mutation).
- New capability = new `Command` + dispatch arm + unit test + MCP tool +
  CLI flag + menu entry. No stubs presented as done.
- `plan/` is gitignored local-only oracle material; `docs/` is committed truth.
