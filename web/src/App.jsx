import React, { useEffect, useRef, useState } from 'react';
import L from 'leaflet';

const VIENNA = [48.2082, 16.3738];

async function api(path, params = {}) {
  const qs = new URLSearchParams(params).toString();
  const r = await fetch(`/api/${path}${qs ? '?' + qs : ''}`);
  const text = await r.text();
  try {
    return { ok: r.ok, data: JSON.parse(text) };
  } catch {
    return { ok: r.ok, data: { raw: text } };
  }
}

export function App() {
  const mapEl = useRef(null);
  const mapRef = useRef(null);
  const overlayRef = useRef(null);
  const [layers, setLayers] = useState([]);
  const [log, setLog] = useState(['qgiscraft console ready.']);
  const [query, setQuery] = useState('Stephansplatz');
  const [tag, setTag] = useState('amenity=pharmacy');

  const say = (line) => setLog((prev) => [...prev.slice(-49), line]);

  const refreshLayers = async () => {
    const { ok, data } = await api('layers');
    if (!ok) {
      say('layers failed: ' + JSON.stringify(data).slice(0, 200));
      return;
    }
    setLayers(data.layers || []);
    say(`layers: ${(data.layers || []).length}`);
  };

  const reloadFeatures = async () => {
    const map = mapRef.current;
    if (!map) return;
    overlayRef.current?.clearLayers();
    const group = L.layerGroup();
    const { ok, data } = await api('features');
    if (!ok) {
      say('features failed');
      return;
    }
    const feats = data.features || [];
    for (const f of feats) {
      if (typeof f.lng !== 'number' || typeof f.lat !== 'number') continue;
      const geom = f.geom_json ? (() => { try { return JSON.parse(f.geom_json); } catch { return null; } })() : null;
      if (geom && geom.type === 'LineString') {
        const latlngs = (geom.coordinates || []).map(([lng, lat]) => [lat, lng]);
        L.polyline(latlngs, { color: '#4cc9f0', weight: 2 }).bindTooltip(f.name).addTo(group);
      } else if (geom && geom.type === 'Polygon') {
        const ring = (geom.coordinates || [])[0] || [];
        const latlngs = ring.map(([lng, lat]) => [lat, lng]);
        L.polygon(latlngs, { color: '#4cc9f0', weight: 2, fillOpacity: 0.25 }).bindTooltip(f.name).addTo(group);
      } else {
        L.circleMarker([f.lat, f.lng], { radius: 5, color: '#4cc9f0', fillOpacity: 0.9 }).bindTooltip(f.name).addTo(group);
      }
    }
    group.addTo(map);
    overlayRef.current = group;
    say(`features: ${feats.length} on map`);
    if (feats.length > 0) {
      const lats = feats.map((f) => f.lat);
      const lngs = feats.map((f) => f.lng);
      map.fitBounds([[Math.min(...lats), Math.min(...lngs)], [Math.max(...lats), Math.max(...lngs)]], { padding: [24, 24] });
    }
  };

  const doGeocode = async (live) => {
    const { ok, data } = await api('geocode', { query, limit: 5, live: live ? 'true' : 'false' });
    say('geocode: ' + JSON.stringify(data).slice(0, 300));
  };

  const doIngest = async () => {
    const [k, v] = tag.split('=');
    if (!k || !v) {
      say('tag must look like key=value');
      return;
    }
    say(`overpass ${k}=${v} live, small Vienna bbox...`);
    const { ok, data } = await api('overpass', {
      key: k.trim(), value: v.trim(),
      south: 48.20, west: 16.36, north: 48.22, east: 16.39,
      live: 'true', ingest: 'true',
    });
    say('ingest: ' + JSON.stringify(data && data.ingest ? data.ingest : data).slice(0, 300));
    await refreshLayers();
    await reloadFeatures();
  };

  useEffect(() => {
    const map = L.map(mapEl.current).setView(VIENNA, 13);
    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 19,
      attribution: '(c) OpenStreetMap contributors',
    }).addTo(map);
    mapRef.current = map;
    overlayRef.current = L.layerGroup().addTo(map);
    refreshLayers().then(reloadFeatures);
    return () => {
      map.remove();
      mapRef.current = null;
    };
  }, []);

  return (
    <div className="layout">
      <div className="header">
        <h1>qgiscraft console</h1>
        <span className="status">Track B seed — same DB as the native app + MCP. Tiles: (c) OpenStreetMap contributors.</span>
      </div>
      <div className="sidebar">
        <h2>Layers</h2>
        {layers.map((l) => (
          <div className="layer-row" key={l.id}>
            <span>{l.name}</span>
            <span>{l.features}</span>
          </div>
        ))}
        <button onClick={async () => { await refreshLayers(); await reloadFeatures(); }}>Refresh</button>

        <h2>Geocode</h2>
        <input type="text" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Stephansplatz" />
        <div>
          <button onClick={() => doGeocode(false)}>Demo</button>
          <button onClick={() => doGeocode(true)}>Live</button>
        </div>

        <h2>Overpass ingest</h2>
        <input type="text" value={tag} onChange={(e) => setTag(e.target.value)} placeholder="amenity=pharmacy" />
        <div>
          <button onClick={doIngest}>Fetch + ingest (Innere Stadt)</button>
        </div>

        <h2>Server map</h2>
        <div>
          <button onClick={() => window.open('/api/preview.png?width=1280&height=860', '_blank')}>Open /api/preview.png</button>
        </div>
      </div>
      <div className="map-wrap">
        <div id="map" ref={mapEl}></div>
      </div>
      <div className="log">{log.join('\n')}</div>
    </div>
  );
}
