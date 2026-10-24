//! Overpass: fenced tag-query builder + keyless live execute.
//! No raw QL passthrough in v1 (injection surface, deliberately deferred).
//! Live path never panics: every network/parse failure is a `GisError`.
use crate::error::GisError;
use crate::geo::Bbox;
use serde::{Deserialize, Serialize};

/// One lightweight hit distilled from an Overpass element.
/// `geom` carries full geometry when the element has it: nodes are Points,
/// open ways LineStrings, closed ways (>=4 pts, first==last) Polygons.
/// `lng`/`lat` is always a valid centroid for lists and labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverpassHit {
    pub kind: String,
    pub name: String,
    pub lng: f64,
    pub lat: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geom: Option<serde_json::Value>,
}

/// Fence for OSM tag keys/values: no quotes, semicolons, or control chars
/// (we generate the quoting; user input must not break out of it).
fn require_tag_part(label: &str, raw: &str) -> Result<String, GisError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(GisError::InvalidInput(format!("{label} must not be empty")));
    }
    if trimmed.chars().count() > 64 {
        return Err(GisError::InvalidInput(format!("{label} exceeds 64 chars")));
    }
    let bad = trimmed
        .chars()
        .any(|c| c == '"' || c == ';' || c == '\'' || c.is_control());
    if bad {
        return Err(GisError::InvalidInput(format!(
            "{label} must not contain quotes, semicolons, or control chars"
        )));
    }
    Ok(trimmed.to_string())
}

/// Build a tag query: nodes + ways + relations with full geometry,
/// capped at 100. `out geom` returns per-way node coordinates; nodes keep
/// their lat/lon. Pure, offline, deterministic.
pub fn build_tag_query(key: &str, value: &str, bbox: Option<Bbox>) -> Result<String, GisError> {
    let key = require_tag_part("key", key)?;
    let value = require_tag_part("value", value)?;
    let area = match bbox {
        Some(b) => format!("({},{},{},{})", b.min_lat, b.min_lng, b.max_lat, b.max_lng),
        None => String::new(),
    };
    Ok(format!(
        "[out:json][timeout:25];(node[\"{key}\"=\"{value}\"]{area};way[\"{key}\"=\"{value}\"]{area};relation[\"{key}\"=\"{value}\"]{area};);out geom 100;"
    ))
}

/// Distill one Overpass element into a hit. Pure, offline, testable.
/// Ways with a `geometry` array become LineString/Polygon; nodes and
/// center-fallback elements become Points.
pub fn element_to_hit(el: &serde_json::Value) -> Option<OverpassHit> {
    let kind = el
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("node")
        .to_string();
    let name = el
        .get("tags")
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("(unnamed)")
        .to_string();
    // Full geometry first: array of {lat, lon}.
    if let Some(geom) = el.get("geometry").and_then(|g| g.as_array()) {
        let mut coords: Vec<(f64, f64)> = Vec::new();
        for pt in geom {
            match (
                pt.get("lon").and_then(|n| n.as_f64()),
                pt.get("lat").and_then(|n| n.as_f64()),
            ) {
                (Some(lng), Some(lat))
                    if (-180.0..=180.0).contains(&lng) && (-90.0..=90.0).contains(&lat) =>
                {
                    coords.push((lng, lat))
                }
                _ => continue,
            }
        }
        if coords.len() >= 2 {
            let closed = coords.len() >= 4 && coords.first() == coords.last();
            let (flat, ty): (Vec<serde_json::Value>, &str) = if closed {
                let ring: Vec<serde_json::Value> = coords
                    .iter()
                    .map(|(lng, lat)| serde_json::json!([lng, lat]))
                    .collect();
                (vec![serde_json::Value::Array(ring)], "Polygon")
            } else {
                let line: Vec<serde_json::Value> = coords
                    .iter()
                    .map(|(lng, lat)| serde_json::json!([lng, lat]))
                    .collect();
                (line, "LineString")
            };
            let (clng, clat) = centroid(&coords);
            let geom = if ty == "Polygon" {
                serde_json::json!({"type": "Polygon", "coordinates": flat})
            } else {
                serde_json::json!({"type": "LineString", "coordinates": flat})
            };
            return Some(OverpassHit {
                kind,
                name,
                lng: clng,
                lat: clat,
                geom: Some(geom),
            });
        }
        // Degenerate geometry (< 2 valid pts): fall through to center.
    }
    // Nodes carry lat/lon; relations without geometry carry center.
    let (lat, lng) = match (
        el.get("lat").and_then(|n| n.as_f64()),
        el.get("lon").and_then(|n| n.as_f64()),
    ) {
        (Some(lat), Some(lng)) => (lat, lng),
        _ => match (
            el.get("center")
                .and_then(|c| c.get("lat"))
                .and_then(|n| n.as_f64()),
            el.get("center")
                .and_then(|c| c.get("lon"))
                .and_then(|n| n.as_f64()),
        ) {
            (Some(lat), Some(lng)) => (lat, lng),
            _ => return None,
        },
    };
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return None;
    }
    Some(OverpassHit {
        kind,
        name,
        lng,
        lat,
        geom: None,
    })
}

/// Arithmetic centroid. Callers guarantee non-empty.
fn centroid(coords: &[(f64, f64)]) -> (f64, f64) {
    let n = coords.len() as f64;
    let (sx, sy) = coords
        .iter()
        .fold((0.0, 0.0), |(x, y), (lng, lat)| (x + lng, y + lat));
    (sx / n, sy / n)
}

/// Overpass interpreter URL (keyless public instance).
/// Override with `QGISCRAFT_OVERPASS_URL` (own mirror recommended).
pub fn overpass_url() -> String {
    std::env::var("QGISCRAFT_OVERPASS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://overpass-api.de/api/interpreter".to_string())
}

/// Execute a builder-produced QL against the Overpass API.
/// Keyless; 60s timeout; typed errors on status/parse.
pub async fn execute_ql(ql: &str) -> Result<Vec<OverpassHit>, GisError> {
    if ql.trim().is_empty() {
        return Err(GisError::InvalidInput("ql must not be empty".to_string()));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("qgiscraft-mcp/0.1 (Overpass; OSM attribution)")
        .build()
        .map_err(|e| GisError::Offline(format!("http client: {e}")))?;
    let resp = client
        .post(overpass_url())
        .body(ql.to_string())
        .send()
        .await
        .map_err(|e| GisError::Offline(format!("overpass unreachable: {e}")))?;
    if !resp.status().is_success() {
        return Err(GisError::Offline(format!(
            "overpass status {}",
            resp.status().as_u16()
        )));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| GisError::Protocol(format!("overpass json: {e}")))?;
    let elements = body
        .get("elements")
        .and_then(|e| e.as_array())
        .ok_or_else(|| GisError::Protocol("overpass: no elements array".to_string()))?;
    let mut hits = Vec::new();
    for el in elements.iter().take(100) {
        if let Some(hit) = element_to_hit(el) {
            hits.push(hit);
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_is_deterministic_offline() {
        let q = build_tag_query("amenity", "pharmacy", None);
        assert!(q.is_ok());
        if let Ok(ql) = q {
            assert!(ql.contains("amenity"));
            assert!(ql.contains("pharmacy"));
            assert!(ql.contains("out geom 100"));
        }
    }

    #[test]
    fn open_way_becomes_linestring() {
        let el = serde_json::json!({
            "type": "way", "id": 1,
            "tags": {"name": "Ring"},
            "geometry": [
                {"lat": 48.20, "lon": 16.30},
                {"lat": 48.21, "lon": 16.31},
                {"lat": 48.22, "lon": 16.32},
            ]
        });
        let hit = element_to_hit(&el);
        assert!(hit.is_some());
        if let Some(hit) = hit {
            let g = hit.geom.unwrap_or(serde_json::Value::Null);
            assert_eq!(g.get("type").and_then(|t| t.as_str()), Some("LineString"));
            assert_eq!(
                g.get("coordinates")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len()),
                Some(3)
            );
        }
    }

    #[test]
    fn closed_way_becomes_polygon() {
        let el = serde_json::json!({
            "type": "way", "id": 2,
            "tags": {"leisure": "park"},
            "geometry": [
                {"lat": 48.20, "lon": 16.30},
                {"lat": 48.20, "lon": 16.32},
                {"lat": 48.22, "lon": 16.32},
                {"lat": 48.20, "lon": 16.30},
            ]
        });
        let hit = element_to_hit(&el);
        assert!(hit.is_some());
        if let Some(hit) = hit {
            let g = hit.geom.unwrap_or(serde_json::Value::Null);
            assert_eq!(g.get("type").and_then(|t| t.as_str()), Some("Polygon"));
        }
    }

    #[test]
    fn degenerate_and_hostile_elements_rejected() {
        let one_pt = serde_json::json!({
            "type": "way", "geometry": [{"lat": 48.2, "lon": 16.3}]
        });
        // Single point, no lat/lon or center: nothing to distill.
        assert!(element_to_hit(&one_pt).is_none());
        let bad = serde_json::json!({"type": "node", "lat": 999.0, "lon": 16.3});
        assert!(element_to_hit(&bad).is_none());
    }

    #[test]
    fn builder_fences_injection() {
        assert!(build_tag_query("amenity\";", "x", None).is_err());
        assert!(build_tag_query("", "x", None).is_err());
        assert!(build_tag_query("amenity", "", None).is_err());
    }
}
