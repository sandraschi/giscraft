//! Overpass: fenced tag-query builder + keyless live execute.
//! No raw QL passthrough in v1 (injection surface, deliberately deferred).
//! Live path never panics: every network/parse failure is a `GisError`.
use crate::error::GisError;
use crate::geo::Bbox;
use serde::{Deserialize, Serialize};

/// One lightweight hit distilled from an Overpass element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverpassHit {
    pub kind: String,
    pub name: String,
    pub lng: f64,
    pub lat: f64,
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

/// Build a tag query: nodes + ways + relations with center, capped at 100.
/// Pure, offline, deterministic. Returned QL is the agent-visible artifact.
pub fn build_tag_query(key: &str, value: &str, bbox: Option<Bbox>) -> Result<String, GisError> {
    let key = require_tag_part("key", key)?;
    let value = require_tag_part("value", value)?;
    let area = match bbox {
        Some(b) => format!("({},{},{},{})", b.min_lat, b.min_lng, b.max_lat, b.max_lng),
        None => String::new(),
    };
    Ok(format!(
        "[out:json][timeout:25];(node[\"{key}\"=\"{value}\"]{area};way[\"{key}\"=\"{value}\"]{area};relation[\"{key}\"=\"{value}\"]{area};);out center 100;"
    ))
}

/// Execute a builder-produced QL against the public Overpass API.
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
        .post("https://overpass-api.de/api/interpreter")
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
        let kind = el
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("node")
            .to_string();
        // Nodes carry lat/lon; ways/relations carry center.lat/center.lon.
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
                _ => continue,
            },
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
            continue;
        }
        let name = el
            .get("tags")
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("(unnamed)")
            .to_string();
        hits.push(OverpassHit {
            kind,
            name,
            lng,
            lat,
        });
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
            assert!(ql.contains("out center 100"));
        }
    }

    #[test]
    fn builder_fences_injection() {
        assert!(build_tag_query("amenity\";", "x", None).is_err());
        assert!(build_tag_query("", "x", None).is_err());
        assert!(build_tag_query("amenity", "", None).is_err());
    }
}
