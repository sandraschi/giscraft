# Data ecosystem: sources, standards, and where your maps go

giscraft reads the world's free map and writes your own layers. This page
explains where data comes from, which standards matter, and where your
annotations can live after they leave this app.

## Data sources (in)

| Source | What | Access |
|---|---|---|
| OpenStreetMap planet / Geofabrik extracts | The whole map as files (planet ~70 GB, Austria ~500 MB, PBF format) | Download; our PBF bulk loader is planned, Overpass covers most needs meanwhile |
| Overpass API | Live queries: "all X in bbox Y" (Overpass QL) | Built in (Download dialog, `overpass.query`); public instances are rate-limited, self-host for heavy use |
| Photon / Nominatim | Address <-> coordinates (geocoding) | Built in (Geocode dialog, `geocode.search`); keyless |
| Your files | GeoJSON (RFC 7946), CSV with lng/lat columns (KML/GPX planned) | Vector > Add Vector/Delimited Text Layer, or `layer.import` |
| XYZ tiles | Pre-rendered map pictures under your vectors | Built-in OSM default + custom tile layers (Raster menu) and the Web preset gallery |

## Standards that matter (GTFS and friends)

GTFS is the *transit* world's interchange standard (stops, routes,
timetables - published by agencies like Verkehrsverbund, consumed by every
routing app). The map world has an equivalent at every layer:

- **OSM data model**: nodes (points), ways (lines/polygons), relations,
  plus free-form `key=value` tags (`amenity=pharmacy`). A documented
  folksonomy (OSM wiki), not a top-down schema like GTFS.
- **GeoJSON (RFC 7946)**: the interchange file. Our import/export speaks it;
  uMap, GitHub, and half the web consume it.
- **XYZ tiles / WMS / PMTiles**: how map *pictures* are served (raster
  squares, OGC services, single-file vector tiles).
- **EPSG:4326 vs EPSG:3857**: GPS coordinates (stored) vs web-map
  projection (drawn). We convert between them on every frame.
- **ODbL license**: OSM data is free *with share-alike* - publish derived
  data openly and attribute contributors (the app shows the (c) notice).
  Never copy from Google Maps into OSM; their license forbids it.

## Where your annotations go (out)

Rule of thumb: *facts about the world* belong in OpenStreetMap;
*your view of the world* belongs on your own map or files.

- **Back into OSM** (real, verifiable, on-the-ground facts: this shop
  exists, that one closed). Tools: the iD editor on openstreetmap.org
  (easiest), JOSM (power), StreetComplete / Every Door (mobile, game-like
  yes/no quests). Our app does **not** upload to OSM yet - that needs your
  OSM account plus the changeset API plus human review before every upload
  (nobody wants bots vandalizing the map). Tracked in `gaps.md`.
- **uMap ([umap.openstreetmap.fr](https://umap.openstreetmap.fr))** - your
  personal map on OSM basemaps: shortlists, notes, colors, shareable links.
  Takes GeoJSON, which is exactly our Export output. Works today.
- **Open-data portals / your own site** - GeoJSON on GitHub, a city portal,
  a blog embed. Same export path.

## Walk, photograph, describe (crossconnect sketch)

The "go for a walk, photograph sights, describe them" loop already has
names: **Mapillary / KartaView** (street-level photo sequences used to
verify map data), **StreetComplete** (OSM gaps as on-foot quests),
**Wikimedia Commons** (geotagged sight photos with descriptions).

A useful crossconnect with us: walk mode records GPS track + timestamped
photos + notes -> lands here as a point layer with photo attachments ->
export splits two ways (sights to Commons, verified POIs toward OSM after
your review). Missing on our side today: features carry name/coords/geometry
only - no photo attachments, no field-capture flow. That lands before any
OSM-upload feature, since verified local data is what makes uploads
trustworthy.
