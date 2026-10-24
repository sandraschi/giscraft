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

# Naked-PC dist: release binaries + docs, zipped (no installer yet).
# Output: dist/qgiscraft-<version>-win64.zip (gitignored).
dist: build-release
    $repo = (Get-Location).Path; $ver = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value; $out = Join-Path $repo "dist"; $stage = Join-Path $out "qgiscraft-$ver-win64"; New-Item -ItemType Directory -Path $stage -Force | Out-Null; Copy-Item (Join-Path $repo "target/release/qgiscraft-app.exe") $stage; Copy-Item (Join-Path $repo "target/release/qgiscraft-mcp.exe") $stage; Copy-Item (Join-Path $repo "target/release/qgiscraft-web.exe") $stage; Copy-Item (Join-Path $repo "README.md") $stage; Copy-Item (Join-Path $repo "INSTALL.md") $stage; Copy-Item (Join-Path $repo "CHANGELOG.md") $stage; Set-Content -Path (Join-Path $stage "START-HERE.txt") -Value "qgiscraft-mcp $ver (Windows 64-bit)`r`n`r`n1. Double-click qgiscraft-app.exe for the map (creates qgiscraft.db next door).`r`n2. Details: README.md, INSTALL.md.`r`n3. MCP: qgiscraft-mcp.exe mcp (stdio). Web: qgiscraft-web.exe (http://127.0.0.1:11254).`r`n"; $zip = Join-Path $out "qgiscraft-$ver-win64.zip"; if (Test-Path $zip) { Remove-Item $zip }; Compress-Archive -Path $stage -DestinationPath $zip; Write-Host "dist: $zip"
