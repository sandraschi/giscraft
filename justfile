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

# Production build (GUI + MCP binaries)
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace
