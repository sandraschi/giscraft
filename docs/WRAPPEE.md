# QGIS (the wrappee)

[QGIS](https://qgis.org) is the free and open-source desktop geographic
information system, GPLv2+. It is the reference `giscraft-mcp` clones
black-box: menu layout, docks, canvas behavior, shortcuts, Processing
Toolbox, Print Layout. No QGIS code is used or needed here.

- Homepage: <https://qgis.org>
- Docs: <https://docs.qgis.org>
- Plugin repository: <https://plugins.qgis.org>
- Source: <https://github.com/qgis/QGIS>
- Python API (PyQGIS): <https://www.qgis.org/pyqgis/>
- Community: [GIS StackExchange](https://gis.stackexchange.com) (`qgis` tag),
  [community forum](https://forum.qgis.org), local user groups per country.
- Our oracle install: QGIS 3.44 LTR via `winget install -e --id OSGeo.QGIS_LTR`.

QGIS plugins (see [docs/plugins.md](plugins.md)) are Python programs against
the PyQGIS API, distributed as zips through plugins.qgis.org and the in-app
Plugin Manager. We do not run them: our MCP tools are the plugin interface
(the capability lands as a Command, not a hosted runtime).
