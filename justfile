set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# qgiscraft-mcp automation (rust-devices-mcp pattern + designcraft xtask).
# Every recipe prefixes cargo via USERPROFILE so NSSM / IDE shells work
# even when .cargo/bin is not on the inherited PATH.

default:
    @just --list

# Strict ArtCraft lint gate across all workspace members
check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit + integration tests
test:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo test --workspace

# Formatting gate
fmt-check:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo fmt --all --check

# Designcraft-style gates via xtask
ci:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p xtask -- ci

parity:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p xtask -- parity

layers:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p xtask -- layers

# Start the MCP server locally over stdio (logs on stderr, JSON-RPC on stdout)
run-mcp:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; $env:RUST_LOG = "debug"; cargo run -p qgiscraft-mcp -- mcp

# CLI smoke: status + layer list without MCP
run-cli:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p qgiscraft-mcp -- status --json; cargo run -p qgiscraft-mcp -- layer-list --json

# Native egui app (Track A clone)
run-app:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p qgiscraft-app --

# Headless GUI-path proof: opens/seeds/lists the project DB, no window
run-app-smoke:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p qgiscraft-app -- --smoke

# Thin web driver (Track B seed): sidecar API + Vite console
run-web:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p qgiscraft-web

# Vite dev console on :11253 (proxies /api to the sidecar on :11254)
run-console:
    npm --prefix web run dev

# Production build (GUI + MCP binaries)
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace
