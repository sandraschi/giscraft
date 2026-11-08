# Glossary

Plain-language definitions for the terms this project throws around.
Entries marked **(app)** are giscraft concepts; the rest is shared GIS vocabulary.

**bbox** - Bounding box: a rectangle on the map given as
`min_lng, min_lat, max_lng, max_lat` (e.g. `48.20, 16.36, 48.22, 16.39`
frames the Innere Stadt). Used to limit Overpass queries and to fit the view.

**changeset** - A bundle of edits uploaded to OpenStreetMap in one go, with a
comment ("added opening hours"). OSM never accepts anonymous drive-by edits;
everything lands as a reviewed changeset under your account.

**CRS** - Coordinate Reference System: the rulebook that turns
round-earth coordinates into flat-map positions. Getting it wrong puts
Vienna in the Atlantic.

**egui / eframe** - The Rust immediate-mode GUI toolkit our native app is
built with (`egui` draws, `eframe` runs the window). Immediate-mode means
the UI is redrawn from scratch every frame - simple to reason about, no
hidden widget state.

**EPSG:3857** - "Web Mercator", the projection every web map (and our
canvas) draws in. Distorts areas toward the poles but makes zoom tiles square.

**EPSG:4326** - Plain GPS coordinates (latitude/longitude in degrees, WGS84).
This is how features are stored; the canvas reprojects to 3857 for display.

**feature** - One thing on the map: a point (pharmacy), line (street), or
polygon (park), plus a name and attributes. GeoJSON's word for it too.

**GeoJSON** - The standard text format for map features (RFC 7946). Our
import/export format and the language our web console speaks.

**Geofabrik** - Service offering daily OpenStreetMap planet extracts cut by
region/country (e.g. all of Austria as one downloadable file). The bulk
alternative to asking Overpass for everything.

**GML / KML / GPX** - XML-based geo formats (GML is OGC's, KML is Google
Earth's, GPX is GPS tracks from hikes and bike rides). Import planned;
GeoJSON and CSV work today.

**Identify** - QGIS's "click the map, tell me what's here" tool.
In giscraft: hovering shows a tooltip; clicking selects the feature's layer
and pins name + coordinates to the status bar.

**id (Overpass)** - Not to confuse with the iD editor: Overpass also has a
naming collision-free numeric identity per object, but in practice we
identify features by name + layer.

**iD editor** - The beginner-friendly in-browser OpenStreetMap editor on
openstreetmap.org. The easiest way to fix a wrong shop or add a missing one.

**JOSM** - The power-user OpenStreetMap editor (Java desktop app). What you
graduate to when iD feels slow and you have hundreds of edits to review.

**layer** - A named set of features drawn together ("amenity-pharmacy",
"Wiener Linien"). Has a geometry type, a visibility switch, and a color.
The Layers panel lists them top-to-bottom as drawn bottom-to-top.

**MCP (Model Context Protocol)** - The open protocol (Anthropic, 2024) that
lets AI assistants call tools. Our server speaks it over stdio, so Claude
can query and build maps the same way the GUI does.

**MCPB** - A packaged MCP server bundle (`.mcpb` file) installable with one
click. Our future distribution format; not built yet.

**Mercator / Web Mercator** - See EPSG:3857. (Gerhard Mercator, 1569; the
web variant cuts off at ±85° latitude, which is why Greenland looks fine
and Antarctica is missing.)

**node / way / relation** - OpenStreetMap's three primitives: a node is a
point, a way is an ordered list of nodes (a road, a building outline), a
relation groups them (a bus route, a multipolygon). Everything else is tags.

**NSIS** - The Windows installer builder. Relevant when we ship a real
setup.exe; today you run from source.

**ODbL** - Open Database License: OSM data's license. You may use it freely,
but published derivatives must stay open and credit contributors
(hence the (c) OpenStreetMap line on our canvas).

**Overpass API / Overpass QL** - A read-only query service over the live
OpenStreetMap database, plus its query language (`node["amenity"="pharmacy"](bbox);out;`).
Free public instances exist but rate-limit; heavy users self-host.
Our Download dialog is a friendly form over one Overpass query.

**Photon / Nominatim** - Place search engines built on OSM data.
Nominatim is the reference implementation; Photon (Komoot) is the fast
API-first one we use for geocoding ("Stephansplatz" -> coordinates).
Both keyless for light use.

**PMTiles / MBTiles** - Single-file formats for map tiles (vector and
raster). A whole country's tiles in one file, servable from static hosting.

**POI** - Point of interest: a named spot (pharmacy, café, ATM, viewpoint).
Most of what our demo layers contain.

**Processing Toolbox** - QGIS's (and now our) algorithm panel: small
geodata tools (buffer, download, geocode) with dialogs, runnable one by one.

**QGIS** - The free, open-source desktop GIS we clone the *behavior* of
(menus, docks, canvas conventions). Clean-room: no QGIS code or assets here.

**GISCRAFT_DB** - Env var pointing at our project database (default
`giscraft.db` in the working directory). The file *is* the project.

**raster / vector** - The two map data kinds. Vector = geometry with
coordinates (points, lines, polygons: crisp at any zoom). Raster = pixels
(satellite imagery, hillshades, and our XYZ basemap tiles: blurry when
over-zoomed).

**scratch layer** - A temporary, empty, in-memory-style layer for sketching
or as an ingest target. QGIS has "New Scratch Layer"; so do we (Database menu).

**shapefile / Shapefile** - The 1990s ESRI format that refuses to die
(actually 3+ files: `.shp` + `.shx` + `.dbf` minimum). Ubiquitous in
government open data; supported via GDAL in QGIS, not yet natively here.

**stdio** - Standard input/output: how our MCP server talks (JSON-RPC lines
on stdin/stdout). Simple, scriptable, no network port needed.

**tag** - An OSM `key=value` label, e.g. `amenity=pharmacy` or
`shop=bicycle`. Tags are what you actually query for; the art is knowing
which tags mappers really use (see `docs/WORKFLOWS.md`).

**tile / XYZ tiles** - Pre-rendered 256x256 map squares addressed by
`z/x/y` (zoom/column/row). Our canvas fetches them on demand and caches
them; `is_usable_template` rejects broken tile URLs before they can blank
the map.

**WMS / WMTS / WFS** - OGC web-service standards for maps (rendered images,
cached tiles, raw vector features respectively). Supported in QGIS;
in giscraft only XYZ tile endpoints so far.

**WKT / WKB** - Well-Known Text/Binary: compact geometry encodings
(`POINT(16.3 48.2)`). What databases (PostGIS/SpatiaLite) store natively.
