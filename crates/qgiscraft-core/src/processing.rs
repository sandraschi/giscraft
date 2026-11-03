//! First real geoprocessing: buffer points into circle polygons.
//! Pure math, no I/O: great-circle destination points on the WGS84 mean
//! radius, closed rings, typed errors. The UI and (later) MCP thinly wrap
//! `buffer_features`, which stays unit-testable without a database.
use crate::error::GisError;
use crate::import::RawFeature;

const EARTH_R_M: f64 = 6_371_008.8;

/// Great-circle destination: point `dist_m` meters from (`lng`,`lat`) on
/// `bearing_deg` (0 = north, clockwise). All inputs fenced.
pub fn destination_point(
    lng: f64,
    lat: f64,
    bearing_deg: f64,
    dist_m: f64,
) -> Result<(f64, f64), GisError> {
    if !lng.is_finite() || !lat.is_finite() || !bearing_deg.is_finite() || !dist_m.is_finite() {
        return Err(GisError::InvalidInput(
            "destination inputs must be finite".to_string(),
        ));
    }
    if dist_m <= 0.0 || dist_m > 1_000_000.0 {
        return Err(GisError::InvalidInput(
            "distance must be 0..1000000 m".to_string(),
        ));
    }
    let (lat1, brng) = (lat.to_radians(), bearing_deg.to_radians());
    let ang = dist_m / EARTH_R_M;
    let lat2 = (lat1.sin() * ang.cos() + lat1.cos() * ang.sin() * brng.cos()).asin();
    let lng2 = lng.to_radians()
        + (brng.sin() * ang.sin() * lat1.cos()).atan2(ang.cos() - lat1.sin() * lat2.sin());
    let (mut out_lng, mut out_lat) = (
        (lng2.to_degrees() + 540.0) % 360.0 - 180.0,
        lat2.to_degrees(),
    );
    if !out_lat.is_finite() {
        out_lat = lat.signum() * 90.0;
    }
    if !(-180.0..=180.0).contains(&out_lng) {
        out_lng = out_lng.clamp(-180.0, 180.0);
    }
    Ok((out_lng, out_lat.clamp(-90.0, 90.0)))
}

/// Buffer every point feature into an N-gon approximating a circle.
/// Non-point features (lines/polygons) pass through unchanged. Returns the
/// new feature set with `geom_json` Polygons and centroid lng/lat.
pub fn buffer_features(
    features: &[RawFeature],
    radius_m: f64,
    segments: usize,
) -> Result<Vec<RawFeature>, GisError> {
    if radius_m <= 0.0 || !radius_m.is_finite() || radius_m > 1_000_000.0 {
        return Err(GisError::InvalidInput(
            "radius must be 0..1000000 m".to_string(),
        ));
    }
    let segments = segments.clamp(8, 256);
    let mut out = Vec::new();
    for f in features {
        if f.geom_json.is_some() {
            out.push(f.clone());
            continue;
        }
        let mut ring: Vec<serde_json::Value> = Vec::new();
        for i in 0..segments {
            let bearing = 360.0 * i as f64 / segments as f64;
            let (lng, lat) = destination_point(f.lng, f.lat, bearing, radius_m)?;
            ring.push(serde_json::json!([lng, lat]));
        }
        if let Some(first) = ring.first().cloned() {
            ring.push(first);
        }
        let geom = serde_json::json!({"type": "Polygon", "coordinates": [ring]});
        out.push(RawFeature {
            name: f.name.clone(),
            kind: "way".to_string(),
            lng: f.lng,
            lat: f.lat,
            geom_json: Some(geom.to_string()),
        });
    }
    if out.is_empty() {
        return Err(GisError::InvalidInput("no features to buffer".to_string()));
    }
    Ok(out)
}

const MAX_GRID_CELLS: usize = 10_000;
const MAX_JOIN_PAIRS: usize = 1_000_000;

/// Meters per degree of latitude (WGS84 mean, good to ~1% outside the poles).
fn meters_per_deg_lat() -> f64 {
    111_320.0
}

fn meters_per_deg_lng(lat: f64) -> f64 {
    111_320.0 * lat.to_radians().cos().abs().max(0.01)
}

fn polygon_feature(
    name: String,
    center: (f64, f64),
    ring: &[(f64, f64)],
) -> Result<RawFeature, GisError> {
    if ring.len() < 3 {
        return Err(GisError::InvalidInput(
            "grid ring needs 3+ points".to_string(),
        ));
    }
    for (x, y) in ring {
        crate::geo::LonLat::new(*x, *y)
            .map_err(|_| GisError::InvalidInput(format!("grid cell out of range ({x}, {y})")))?;
    }
    let inner: Vec<String> = ring.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    let mut closed = inner.clone();
    if let Some(first) = inner.first().cloned() {
        if closed.last() != Some(&first) {
            closed.push(first);
        }
    }
    Ok(RawFeature {
        name,
        kind: "way".to_string(),
        lng: center.0,
        lat: center.1,
        geom_json: Some(format!(
            "{{\"type\":\"Polygon\",\"coordinates\":[{}]}}",
            format!("[{}]", closed.join(","))
        )),
    })
}

/// Build a rect or hex grid over a bbox. `cell_size_m` is the rect edge, or
/// the hex width (flats) for pointy-top hexagons. Pure math, capped at
/// 10k cells. Centers are feature lng/lat; rings are geodesic polygons.
pub fn grid_features(
    south: f64,
    west: f64,
    north: f64,
    east: f64,
    cell_size_m: f64,
    hex: bool,
) -> Result<Vec<RawFeature>, GisError> {
    let bbox = crate::geo::Bbox::new(west, south, east, north)?;
    if !cell_size_m.is_finite() || cell_size_m < 10.0 || cell_size_m > 100_000.0 {
        return Err(GisError::InvalidInput(
            "cell size must be 10..100000 m".to_string(),
        ));
    }
    let mid_lat = (bbox.min_lat + bbox.max_lat) / 2.0;
    let d_lat = cell_size_m / meters_per_deg_lat();
    let d_lng = cell_size_m / meters_per_deg_lng(mid_lat);
    if d_lat <= 0.0 || d_lng <= 0.0 || !d_lat.is_finite() || !d_lng.is_finite() {
        return Err(GisError::InvalidInput("grid step not finite".to_string()));
    }
    let rows = ((bbox.max_lat - bbox.min_lat) / d_lat).ceil() as usize;
    let cols = ((bbox.max_lng - bbox.min_lng) / d_lng).ceil() as usize;
    let cells = rows
        .checked_mul(cols)
        .ok_or_else(|| GisError::InvalidInput("grid too large".to_string()))?;
    if cells == 0 || cells > MAX_GRID_CELLS {
        return Err(GisError::InvalidInput(format!(
            "grid would build {cells} cells (cap {MAX_GRID_CELLS}; grow cell size or shrink bbox)"
        )));
    }
    let mut out = Vec::new();
    if hex {
        // Pointy-top hexes, width = cell_size across the flats.
        let s = cell_size_m / 2.0;
        let w_step = cell_size_m / meters_per_deg_lng(mid_lat);
        let h_step = (1.5 * s) / meters_per_deg_lat();
        let mut row = 0_usize;
        let mut lat = bbox.min_lat + (s / meters_per_deg_lat());
        while lat <= bbox.max_lat {
            let offset = if row % 2 == 1 { w_step / 2.0 } else { 0.0 };
            let mut lng = bbox.min_lng + (cell_size_m / 2.0) / meters_per_deg_lng(mid_lat) + offset;
            let mut col = 0_usize;
            while lng <= bbox.max_lng {
                let mut ring: Vec<(f64, f64)> = Vec::new();
                for k in 0..6 {
                    let bearing = 360.0 * k as f64 / 6.0;
                    ring.push(destination_point(lng, lat, bearing, s)?);
                }
                out.push(polygon_feature(
                    format!("hex-{row}-{col}"),
                    (lng, lat),
                    &ring,
                )?);
                lng += w_step;
                col = col.saturating_add(1);
            }
            lat += h_step;
            row = row.saturating_add(1);
            if row > MAX_GRID_CELLS {
                return Err(GisError::InvalidInput("grid too large".to_string()));
            }
        }
    } else {
        for r in 0..rows {
            for c in 0..cols {
                let x0 = bbox.min_lng + c as f64 * d_lng;
                let y0 = bbox.min_lat + r as f64 * d_lat;
                let x1 = (x0 + d_lng).min(bbox.max_lng);
                let y1 = (y0 + d_lat).min(bbox.max_lat);
                let ring = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
                out.push(polygon_feature(
                    format!("cell-{r}-{c}"),
                    ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
                    &ring,
                )?);
            }
        }
    }
    if out.is_empty() {
        return Err(GisError::InvalidInput("grid produced no cells".to_string()));
    }
    Ok(out)
}

/// Nearest-neighbor join: one hub line per source feature to its closest
/// target feature (haversine). Names carry both ends plus meters.
pub fn nearest_join(
    source: &[RawFeature],
    target: &[RawFeature],
) -> Result<Vec<RawFeature>, GisError> {
    if source.is_empty() || target.is_empty() {
        return Err(GisError::InvalidInput(
            "nearest join needs features on both sides".to_string(),
        ));
    }
    let pairs = source
        .len()
        .checked_mul(target.len())
        .ok_or_else(|| GisError::InvalidInput("join too large".to_string()))?;
    if pairs > MAX_JOIN_PAIRS {
        return Err(GisError::InvalidInput(format!(
            "join would score {pairs} pairs (cap {MAX_JOIN_PAIRS})"
        )));
    }
    let mut out = Vec::new();
    for s in source {
        let mut best: Option<(&RawFeature, f64)> = None;
        for t in target {
            let d = crate::geo::haversine_m(
                crate::geo::LonLat::new(s.lng, s.lat).map_err(|_| {
                    GisError::InvalidInput("source coordinate out of range".to_string())
                })?,
                crate::geo::LonLat::new(t.lng, t.lat).map_err(|_| {
                    GisError::InvalidInput("target coordinate out of range".to_string())
                })?,
            );
            let replace = match best {
                Some((_, bd)) => d < bd,
                None => true,
            };
            if replace {
                best = Some((t, d));
            }
        }
        if let Some((t, d)) = best {
            let line = format!(
                "{{\"type\":\"LineString\",\"coordinates\":[[{},{}],[{},{}]]}}",
                s.lng, s.lat, t.lng, t.lat
            );
            out.push(RawFeature {
                name: format!("{} -> {} ({:.0} m)", s.name, t.name, d),
                kind: "way".to_string(),
                lng: (s.lng + t.lng) / 2.0,
                lat: (s.lat + t.lat) / 2.0,
                geom_json: Some(line),
            });
        }
    }
    if out.is_empty() {
        return Err(GisError::InvalidInput("join produced no lines".to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_km_north_of_stephansplatz() {
        let dest = destination_point(16.3738, 48.2082, 0.0, 1000.0);
        assert!(dest.is_ok());
        if let Ok((lng, lat)) = dest {
            // ~0.009 degrees latitude per km.
            assert!((lat - 48.2082 - 0.009).abs() < 0.001, "lat {lat}");
            assert!((lng - 16.3738).abs() < 0.001, "lng {lng}");
        }
    }

    #[test]
    fn hostile_inputs_rejected() {
        assert!(destination_point(16.3, 48.2, 0.0, 0.0).is_err());
        assert!(destination_point(16.3, 48.2, 0.0, -5.0).is_err());
        assert!(destination_point(f64::NAN, 48.2, 0.0, 10.0).is_err());
        assert!(buffer_features(&[], 100.0, 24).is_err());
    }

    #[test]
    fn buffer_produces_closed_polygons() {
        let pts = vec![RawFeature {
            name: "A".to_string(),
            kind: "node".to_string(),
            lng: 16.3738,
            lat: 48.2082,
            geom_json: None,
        }];
        let out = buffer_features(&pts, 500.0, 24);
        assert!(out.is_ok());
        if let Ok(feats) = out {
            assert_eq!(feats.len(), 1);
            let has_poly = feats[0]
                .geom_json
                .as_ref()
                .is_some_and(|g| g.contains("Polygon"));
            assert!(has_poly);
            // Closed ring: first == last coordinate.
            let mut closed = false;
            if let Some(g) = feats[0].geom_json.as_ref() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(g) {
                    if let Some(ring) = v
                        .get("coordinates")
                        .and_then(|c| c.get(0))
                        .and_then(|r| r.as_array())
                    {
                        assert_eq!(ring.len(), 25);
                        closed = ring.first() == ring.last();
                    }
                }
            }
            assert!(closed);
        }
    }

    fn point(name: &str, lng: f64, lat: f64) -> RawFeature {
        RawFeature {
            name: name.to_string(),
            kind: "point".to_string(),
            lng,
            lat,
            geom_json: None,
        }
    }

    #[test]
    fn rect_grid_covers_bbox() {
        let feats = grid_features(48.20, 16.36, 48.21, 16.37, 500.0, false);
        assert!(feats.is_ok());
        if let Ok(feats) = feats {
            assert!(!feats.is_empty() && feats.len() <= 10_000);
            assert!(feats[0].name.starts_with("cell-"));
            if let Some(g) = feats[0].geom_json.as_ref() {
                assert!(g.contains("Polygon"));
            }
        }
    }

    #[test]
    fn hex_grid_builds_six_sided_cells() {
        let feats = grid_features(48.20, 16.36, 48.22, 16.38, 500.0, true);
        assert!(feats.is_ok());
        if let Ok(feats) = feats {
            assert!(feats[0].name.starts_with("hex-"));
            if let Some(g) = feats[0].geom_json.as_ref() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(g) {
                    if let Some(ring) = v
                        .get("coordinates")
                        .and_then(|c| c.get(0))
                        .and_then(|r| r.as_array())
                    {
                        // 6 vertices + closing repeat.
                        assert_eq!(ring.len(), 7);
                    }
                }
            }
        }
    }

    #[test]
    fn grid_rejects_absurd_sizes() {
        // South above north.
        assert!(grid_features(48.22, 16.36, 48.20, 16.38, 500.0, false).is_err());
        // Below the 10 m floor.
        assert!(grid_features(48.20, 16.36, 48.21, 16.37, 5.0, false).is_err());
        assert!(grid_features(48.20, 16.36, 48.21, 16.37, 500.0, true).is_ok());
        // 10 m cells over the district would blow the 10k cap.
        assert!(grid_features(48.20, 16.36, 48.22, 16.38, 10.0, false).is_err());
    }

    #[test]
    fn nearest_join_picks_closest_and_names_distance() {
        let src = vec![point("bike-1", 16.3700, 48.2000)];
        let tgt = vec![
            point("near-pharm", 16.3710, 48.2005),
            point("far-pharm", 16.4000, 48.2200),
        ];
        let out = nearest_join(&src, &tgt);
        assert!(out.is_ok());
        if let Ok(lines) = out {
            assert_eq!(lines.len(), 1);
            assert!(lines[0].name.contains("near-pharm"));
            assert!(lines[0].name.contains(" m)"));
            if let Some(g) = lines[0].geom_json.as_ref() {
                assert!(g.contains("LineString"));
            }
        }
        assert!(nearest_join(&[], &tgt).is_err());
        assert!(nearest_join(&src, &[]).is_err());
    }
}
