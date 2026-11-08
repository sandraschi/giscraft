# Configuration

All tuning is environment variables. No config files, no registry.

| Variable | Default | Description |
|----------|---------|-------------|
| `GISCRAFT_DB` | `./giscraft.db` | Project database (layers + features). The file IS the project. |
| `GISCRAFT_TILES_URL` | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` | XYZ tile template (`{z}/{x}/{y}` substituted). Point at a mirror or local tileserver to go offline. |
| `GISCRAFT_PHOTON_URL` | `https://photon.komoot.io/api/` | Photon geocoder base (Komoot public instance, keyless). |
| `GISCRAFT_OVERPASS_URL` | `https://overpass-api.de/api/interpreter` | Overpass interpreter (public instance, keyless, rate-limited). Run your own for heavy use. |

## Setting variables

For the native app, set them before launch:

```powershell
$env:GISCRAFT_DB = "D:\maps\wien.db"
.\target\debug\giscraft.exe
```

For Claude Desktop, put them in the `env` block of the server entry
(see [INSTALL.md](../INSTALL.md)). The Settings menu shows the effective
values for the running app (read-only; restart to apply changes).
