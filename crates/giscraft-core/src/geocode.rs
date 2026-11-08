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

/// Photon base URL (keyless). Override with `GISCRAFT_PHOTON_URL`.
pub fn photon_base() -> String {
    std::env::var("GISCRAFT_PHOTON_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://photon.komoot.io/api/".to_string())
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
        .user_agent("giscraft-mcp/0.1 (Photon live; OSM attribution)")
        .build()
        .map_err(|e| GisError::Offline(format!("http client: {e}")))?;
    let resp = client
        .get(photon_base())
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

/// Offline city table for "go to place" in front ends: (names, lat, lng). The first
/// name is the display name, the rest are aliases. Small on purpose: anything not
/// listed goes to the live geocoder, never to a guess.
const CITIES: &[(&[&str], f64, f64)] = &[
    (&["Wien", "vienna"], 48.2082, 16.3738),
    (&["Graz"], 47.0707, 15.4395),
    (&["Salzburg"], 47.8095, 13.0550),
    (&["Innsbruck"], 47.2692, 11.4041),
    (&["Linz"], 48.3069, 14.2858),
    (&["Klagenfurt"], 46.6247, 14.3050),
    (&["Tokyo", "tokio"], 35.6762, 139.6503),
    (&["Kyoto"], 35.0116, 135.7681),
    (&["Osaka"], 34.6937, 135.5023),
    (&["Nara"], 34.6851, 135.8049),
    (&["Berlin"], 52.5200, 13.4050),
    (&["London"], 51.5074, -0.1278),
    (&["Paris"], 48.8566, 2.3522),
    (&["Rome", "roma"], 41.9028, 12.4964),
    (&["New York", "nyc"], 40.7128, -74.0060),
];

/// Look a city up in the offline table. Case-insensitive, exact name or alias.
/// Returns (display name, lat, lng); `None` means "not in the table", not "does not exist".
pub fn city_lookup(name: &str) -> Option<(&'static str, f64, f64)> {
    let q = name.trim().to_lowercase();
    if q.is_empty() {
        return None;
    }
    CITIES
        .iter()
        .find(|(names, _, _)| names.iter().any(|n| n.to_lowercase() == q))
        .map(|(names, lat, lng)| (names[0], *lat, *lng))
}

#[cfg(test)]
mod city_tests {
    use super::city_lookup;

    #[test]
    fn known_cities_and_aliases_resolve() {
        let found = city_lookup("Tokyo");
        assert!(found.is_some(), "tokyo missing from the city table");
        let (n, lat, lng) = found.unwrap_or(("", 0.0, 0.0));
        assert_eq!(n, "Tokyo");
        assert!((lat - 35.68).abs() < 0.1 && (lng - 139.65).abs() < 0.1);
        assert_eq!(city_lookup("  TOKIO ").map(|c| c.0), Some("Tokyo"));
        assert_eq!(city_lookup("vienna").map(|c| c.0), Some("Wien"));
    }

    #[test]
    fn unknown_and_empty_are_none_not_a_guess() {
        assert!(city_lookup("Atlantis").is_none());
        assert!(city_lookup("   ").is_none());
        assert!(city_lookup("48.2, 16.3").is_none());
    }
}
