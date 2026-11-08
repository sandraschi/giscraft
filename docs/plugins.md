# Plugin strategy

QGIS plugins ([plugins.qgis.org](https://plugins.qgis.org)) are Python
programs against the PyQGIS API (`qgis.core` / `qgis.gui`), shipped as zips
with a `metadata.txt` and installed via the in-app Plugin Manager. QGIS
itself recommends writing new capabilities as Processing subplugins rather
than standalone plugins.

## Decision: we do not host plugins; MCP tools ARE the plugin system

A capability that would be a QGIS plugin lands here as:

1. A `Command` variant in `giscraft-core` (typed params, headless test),
2. An MCP tool + CLI subcommand in L2 (same dispatch),
3. A menu entry in the egui app (real action, never a stub label).

Precedent: QuickOSM (the most-installed Overpass plugin) was not ported -
its *capability* was reimplemented natively as `overpass.query`
(fenced builder, dry-run QL, live execute, `--ingest`).

Full PyQGIS compatibility is explicitly out: it would mean embedding a
Python runtime and reimplementing `qgis.core` - a second QGIS, with a second
set of 2005-era semantics. The storytold precedent agrees (the designcraft
family does not run ExtendScript/UXP plugins either).

## If a killer plugin appears

File it as a `docs/gaps.md` item with the capability (not the code) as the
target, e.g. "QuickMapServices basemap picker -> XYZ template presets".
Reimplement the behavior behind our command bus; never vendor the plugin.
