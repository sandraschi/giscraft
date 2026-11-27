# Storytold Beta Readiness TODO - GISCraft

**Target:** Submit to https://getartcraft.com/beta (Tier A Candidate #1 - closest to submission-ready)
**Audited:** 2026-11-21 (16.9k LOC, 148 tests, zero external sibling path deps)
**Reference:** `mcp-central-docs/integrations/storytold-artcraft/BETA_CANDIDATES.md`

---

## Already In Place
- [x] `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md`
- [x] `AGENTS.md`, `docs/target-app-parity.md`, `docs/gaps.md`
- [x] `xtask/Cargo.toml` and `.github/workflows/ci.yml`
- [x] Zero external `../sibling` path dependencies (fresh-clone safe)
- [x] 148 unit/integration tests + headless map previews (`docs/map-preview.png`, `docs/map-bikeshops.png`)

---

## P0 - Hard Submission Blockers (Must Complete Before Beta Form)

- [x] **RFC-0029 trademark & naming audit (`docs/feature-names.md`)**
  - Created `docs/feature-names.md` and scrubbed third-party GIS brand pitch from `README.md`, `main.rs` About dialog, `help_view.rs`, `giscraft-mcp/src/main.rs`, and `crates/giscraft/Cargo.toml` (keeping factual `.qgz`/GeoJSON/GeoPackage/Shapefile interop).
- [x] **Cross-platform CI & release artifacts**
  - Upgraded `.github/workflows/ci.yml` to `ubuntu-latest`, `macos-latest`, `windows-latest` matrix (`cargo xtask ci`).
  - Added `.github/workflows/release.yml` publishing native binaries for Windows/macOS/Linux + `SHA256SUMS.txt`.
- [x] **Add `packaging/env.sh`** (app ID, display name, binary names for Storytold launcher/packaging convention).
- [ ] **SPDX license allowlist verification**
  - Confirm `cargo xtask` checks `Cargo.lock` against Storytold `standards/licences.md` (zero GPL/LGPL/AGPL linked or bundled).

## P1 - Desktop UX & Core GIS Depth

- [ ] **Real desktop UI screenshots (dark + light modes)**
  - Capture full desktop window screenshots (canvas + layers dock + toolbar + menus) in both dark and light themes under `docs/images/` and link them in `README.md` (current PNGs are raw map renders, not full UI).
- [ ] **Core GIS I/O parity**
  - Verify end-to-end GeoJSON, GeoPackage (`.gpkg`), and Shapefile (`.shp`) import/export + basic CRS projection handling (WGS84 / Web Mercator / UTM) so non-OSM users have immediate value.
- [ ] **Sync `AGENTS.md`** with `storytold/craft-contrib/standards/app-agents.md` (RFC-0072) and verify `README.md` removes any sandraschi-specific Track B port references (`11253/11254`) from the main user pitch.

## P2 - Final Verification & Submission

- [ ] **Accessibility check (RFC-0014)**: keyboard navigation for all map/layer commands, numeric inputs for zoom/bbox/coordinates, high-contrast light mode.
- [ ] **Clean fresh-clone build**: clone to a temp directory, run `cargo xtask ci` and `--smoke`, confirm zero warnings and zero reachable `todo!()`/`unimplemented!()`.
- [ ] **Submit** 4-field form at https://getartcraft.com/beta (App name, Email, Repo URL, Description).
