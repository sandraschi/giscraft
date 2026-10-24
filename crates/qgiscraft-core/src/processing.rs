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
}
