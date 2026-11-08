# Control protocol

The running app speaks JSON lines on `127.0.0.1` when started with
`--control <port>` (or `QGISCRAFT_CONTROL_PORT`). Mirrors the storytold
artcraft contract (`designcraft --control`), trimmed to our surface.

- One request object per line, one reply per line. Blank lines skipped.
- Every request must be a JSON object with a string `method`
  (`id` optional, `params` optional).
- Anything else gets ONE error reply and the connection is closed
  (a browser `fetch` to the port cannot smuggle commands in).
- Lines over 4 MiB are rejected; max 16 concurrent connections.
- No auth: enable only while you use it.
- Replies: `{"id", "ok": true, "result"}` or `{"id", "ok": false, "error"}`.

`qgiscraft -- --control 7979`

## Methods

| Method | Params | Effect |
|---|---|---|
| `project.status` | - | db, CRS, layers, center, zoom, selection |
| `layer.add` | `id`, `name`, `source?`, `geom_type?`, `crs?` | Add via app (same DB), refreshes view model |
| `layer.list` | - | Layers + visible + per-layer feature counts |
| `layer.remove` | `id` | Remove + refresh |
| `ui.inspect` | - | version, center, zoom, selection, layers, db, canvas size |
| `map.fit` | `layer_id?` | Fit view next frame (`{"queued": true}` - async) |
| `map.render` | `width?`, `height?`, `tiles?` | Headless BMP render to temp, returns `path` |

`MCP --connect` forwards the four DB-touching tools
(`project.status`, `layer.add/list/remove`) through this channel so the
running app is the single writer; geocode/overpass/features/export stay
local (network or headless reads, no app state involved).

## MCP usage

```json
{ "command": "qgiscraft-mcp.exe", "args": ["mcp", "--connect", "7979"] }
```

With `--connect`, `layer.add` in Claude appears in the app after F5
(the view model refreshes; the canvas repaints live).
