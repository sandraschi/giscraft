set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

# giscraft automation (rust-devices-mcp pattern + designcraft xtask).
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

forge:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p xtask -- forge

layers:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p xtask -- layers

# Start the MCP server locally over stdio (logs on stderr, JSON-RPC on stdout)
run-mcp:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; $env:RUST_LOG = "debug"; cargo run -p giscraft-mcp -- mcp

# CLI smoke: status + layer list without MCP
run-cli:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p giscraft-mcp -- status --json; cargo run -p giscraft-mcp -- layer-list --json

# Native egui app (Track A clone)
run-app:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p giscraft --

# Headless GUI-path proof: opens/seeds/lists the project DB, no window
run-app-smoke:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p giscraft -- --smoke

# Thin web driver (Track B seed): sidecar API + Vite console
run-web:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo run -p giscraft-web

# Vite dev console on :11253 (proxies /api to the sidecar on :11254)
run-console:
    npm --prefix web run dev

# Production build (GUI + MCP binaries)
build-release:
    $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"; cargo build --release --workspace

# Release assets in dist/ (gitignored): the three release exes as plain files, the
# launcher-only giscraft-<version>-windows-x64-portable.zip (same exes) and SHA256SUMS.txt
# over all of them. See mcd standards/rust-artcraft/RELEASE_FACTORY.md.
# Upload: gh release upload v<version> dist/* --clobber
dist: build-release
    $ErrorActionPreference = "Stop"; $out = Join-Path (Get-Location).Path "dist"; if (Test-Path $out) { Remove-Item $out -Recurse -Force }; New-Item -ItemType Directory -Path $out -Force | Out-Null; $exes = "giscraft.exe", "giscraft-mcp.exe", "giscraft-web.exe"; foreach ($e in $exes) { Copy-Item (Join-Path "target/release" $e) $out }; $ver = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value; $zip = "giscraft-$ver-windows-x64-portable.zip"; Compress-Archive -Path ($exes | ForEach-Object { Join-Path $out $_ }) -DestinationPath (Join-Path $out $zip) -Force; $sums = foreach ($e in ($exes + $zip)) { "{0}  {1}" -f (Get-FileHash (Join-Path $out $e) -Algorithm SHA256).Hash.ToLower(), $e }; [IO.File]::WriteAllText((Join-Path $out "SHA256SUMS.txt"), (($sums -join "`n") + "`n"), (New-Object Text.UTF8Encoding $false)); $sums; Write-Host "dist: $out"
