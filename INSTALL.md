# Installing qgiscraft

Pre-release: building from source is currently the only install path.
No NSIS installer and no `.mcpb` bundle exist yet.

## Prerequisites

Install these if you don't have them already:

| Tool | Purpose | Install |
|------|---------|---------|
| Rust (stable) | Build everything | `winget install Rustlang.Rustup` then `rustup-init` |
| MSVC linker | Required: Rust on Windows links with it | Visual Studio Build Tools, "Desktop development with C++" workload |
| Git | Clone the repo | `winget install Git.Git` |
| Claude Desktop | MCP host (tool use only) | [download](https://claude.ai/download) |
| QGIS LTR (optional) | Side-by-side oracle + cross-checks | `winget install -e --id OSGeo.QGIS_LTR` |

> Windows: all installs via [winget](https://learn.microsoft.com/en-us/windows/package-manager/winget/).
> Without the MSVC linker, `cargo build` fails with `link.exe not found` —
> see [Troubleshooting](docs/TROUBLESHOOTING.md).

## Option A — Native app + CLI (recommended today)

```powershell
git clone https://github.com/sandraschi/qgiscraft
cd qgiscraft
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo build -p qgiscraft-app -p qgiscraft-mcp
.\target\debug\qgiscraft-app.exe
```

The app opens (or creates) `qgiscraft.db` in the working directory.
First map in 30 seconds: use the CLI to ingest live pharmacies, then Fit data
in the app (or press F5 after ingesting while it runs):

```powershell
cargo run -q -p qgiscraft-mcp -- overpass-query --key amenity --value pharmacy --south 48.20 --west 16.36 --north 48.22 --east 16.39 --live --ingest --json
```

(Note: the public Overpass API rate-limits; on HTTP 502 wait a minute and retry.)

## Option B — Claude Desktop (MCP tools only)

Build once (Option A), then register the server in
`%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "qgiscraft": {
      "command": "D:\\Dev\\repos\\qgiscraft\\target\\debug\\qgiscraft-mcp.exe",
      "args": ["mcp"],
      "env": { "QGISCRAFT_DB": "D:\\Dev\\repos\\qgiscraft\\qgiscraft.db" }
    }
  }
}
```

Adjust the paths to your checkout. Restart Claude Desktop.

## Verify Installation

App: you see Browser/Layers docks, a tile map after Fit data, `z15.x` in the status bar.
Claude Desktop: type

> "List the project layers with feature counts"

You should see a JSON layer list (possibly just `osm-basemap` on a fresh DB).

## Troubleshooting

See [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) for common issues
(Overpass 502s, tile fetch stalls, `link.exe not found`, DB locked).
