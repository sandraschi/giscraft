# Configuration

All tuning is environment variables. No config files, no registry.

| Variable | Default | Description |
|----------|---------|-------------|
| `QGISCRAFT_DB` | `./qgiscraft.db` | Project database (layers + features). The file IS the project. |
| `QGISCRAFT_TILES_URL` | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` | XYZ tile template (`{z}/{x}/{y}` substituted). Point at a mirror or local tileserver to go offline. |
| `QGISCRAFT_PHOTON_URL` | `https://photon.komoot.io/api/` | Photon geocoder base (Komoot public instance, keyless). |
| `QGISCRAFT_OVERPASS_URL` | `https://overpass-api.de/api/interpreter` | Overpass interpreter (public instance, keyless, rate-limited). Run your own for heavy use. |

## Setting variables

For the native app, set them before launch:

```powershell
$env:QGISCRAFT_DB = "D:\maps\wien.db"
.\target\debug\qgiscraft-app.exe
```

For Claude Desktop, put them in the `env` block of the server entry
(see [INSTALL.md](../INSTALL.md)). The Settings menu shows the effective
values for the running app (read-only; restart to apply changes).
