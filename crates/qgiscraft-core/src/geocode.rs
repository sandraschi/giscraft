//! Geocode: demo default (offline, deterministic), Photon live env-gated.
//! Live path never panics: every network/parse failure becomes `GisError`.
use crate::error::GisError;
use serde::{Deserialize, Serialize};

/// One search hit, demo or live.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub name: String,
    pub lng: f64,
    pub lat: f64,
    pub source: String,
}

/// Offline deterministic demo (Vienna). No network, no env.
pub fn demo_search(query: &str, limit: u8) -> Result<Vec<Hit>, GisError> {
    if query.trim().is_empty() {
        return Err(GisError::InvalidInput(
            "query must not be empty".to_string(),
        ));
    }
    if limit == 0 || limit > 50 {
        return Err(GisError::InvalidInput("limit must be 1..50".to_string()));
    }
    Ok(vec![Hit {
        name: format!("{}, Wien (demo)", query.trim()),
        lng: 16.3738,
        lat: 48.2082,
        source: "demo".to_string(),
    }])
}

/// Live Photon lookup (photon.komoot.io, keyless). Timeouts + parse fenced.
pub async fn photon_search(query: &str, limit: u8) -> Result<Vec<Hit>, GisError> {
    if query.trim().is_empty() {
        return Err(GisError::InvalidInput(
            "query must not be empty".to_string(),
        ));
    }
    if limit == 0 || limit > 50 {
        return Err(GisError::InvalidInput("limit must be 1..50".to_string()));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("qgiscraft-mcp/0.1 (Photon live; OSM attribution)")
        .build()
        .map_err(|e| GisError::Offline(format!("http client: {e}")))?;
    let resp = client
        .get("https://photon.komoot.io/api/")
        .query(&[("q", query.trim()), ("limit", limit.to_string().as_str())])
        .send()
        .await
        .map_err(|e| GisError::Offline(format!("photon unreachable: {e}")))?;
    if !resp.status().is_success() {
        return Err(GisError::Offline(format!(
            "photon status {}",
            resp.status().as_u16()
        )));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| GisError::Protocol(format!("photon json: {e}")))?;
    let features = body
        .get("features")
        .and_then(|f| f.as_array())
        .ok_or_else(|| GisError::Protocol("photon: no features array".to_string()))?;
    let mut hits = Vec::new();
    for feat in features.iter().take(limit as usize) {
        let lng = feat
            .get("geometry")
            .and_then(|g| g.get("coordinates"))
            .and_then(|c| c.get(0))
            .and_then(|n| n.as_f64());
        let lat = feat
            .get("geometry")
            .and_then(|g| g.get("coordinates"))
            .and_then(|c| c.get(1))
            .and_then(|n| n.as_f64());
        let name = feat
            .get("properties")
            .and_then(|p| {
                p.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| query.trim().to_string());
        match (lng, lat) {
            (Some(lng), Some(lat)) => {
                if (-180.0..=180.0).contains(&lng) && (-90.0..=90.0).contains(&lat) {
                    hits.push(Hit {
                        name,
                        lng,
                        lat,
                        source: "photon".to_string(),
                    });
                }
            }
            _ => continue,
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_deterministic_and_offline() {
        let hits = demo_search("Stephansplatz", 5);
        assert!(hits.is_ok());
        if let Ok(hits) = hits {
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].source, "demo");
        }
    }

    #[test]
    fn hostile_inputs_rejected_without_network() {
        assert!(demo_search("", 5).is_err());
        assert!(demo_search("Wien", 0).is_err());
        assert!(demo_search("Wien", 51).is_err());
    }
}
