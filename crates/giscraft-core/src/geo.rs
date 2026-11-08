//! L1 geo primitives: fenced lon/lat, haversine, bbox. No I/O, no UI.
use crate::error::{require_lon_lat, GisError};
use serde::{Deserialize, Serialize};

/// One WGS84 point. Validated at construction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LonLat {
    pub lng: f64,
    pub lat: f64,
}

impl LonLat {
    pub fn new(lng: f64, lat: f64) -> Result<Self, GisError> {
        let (lng, lat) = require_lon_lat(lng, lat)?;
        Ok(Self { lng, lat })
    }
}

/// Axis-aligned bbox in WGS84.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bbox {
    pub min_lng: f64,
    pub min_lat: f64,
    pub max_lng: f64,
    pub max_lat: f64,
}

impl Bbox {
    pub fn new(min_lng: f64, min_lat: f64, max_lng: f64, max_lat: f64) -> Result<Self, GisError> {
        let _ = require_lon_lat(min_lng, min_lat)?;
        let _ = require_lon_lat(max_lng, max_lat)?;
        if min_lng > max_lng || min_lat > max_lat {
            return Err(GisError::InvalidInput(
                "bbox min must be <= max".to_string(),
            ));
        }
        Ok(Self {
            min_lng,
            min_lat,
            max_lng,
            max_lat,
        })
    }

    pub fn contains(&self, p: LonLat) -> bool {
        p.lng >= self.min_lng
            && p.lng <= self.max_lng
            && p.lat >= self.min_lat
            && p.lat <= self.max_lat
    }
}

/// Parse human "lat, lng" text: "48.2082, 16.3738", "48.2082 16.3738",
/// "48.2082; 16.3738", optional N/S/E/W suffix per component ("48.2N, 16.3E").
/// Order is always lat-first. S/W suffixes negate; a suffix contradicting an
/// explicit sign is rejected.
pub fn parse_latlng(text: &str) -> Result<LonLat, GisError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(GisError::InvalidInput(
            "coords must not be empty".to_string(),
        ));
    }
    if trimmed.chars().count() > 64 {
        return Err(GisError::InvalidInput(
            "coords exceeds 64 chars".to_string(),
        ));
    }
    let parts: Vec<&str> = if trimmed.contains(',') {
        trimmed.split(',').collect()
    } else if trimmed.contains(';') {
        trimmed.split(';').collect()
    } else {
        trimmed.split_whitespace().collect()
    };
    if parts.len() != 2 {
        return Err(GisError::InvalidInput(
            "coords need exactly two numbers (lat, lng)".to_string(),
        ));
    }
    let lat = parse_component(parts[0], true)?;
    let lng = parse_component(parts[1], false)?;
    LonLat::new(lng, lat)
}

fn parse_component(raw: &str, is_lat: bool) -> Result<f64, GisError> {
    let bad = || GisError::InvalidInput("bad coordinate component".to_string());
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(bad());
    }
    let (num_part, suffix) = match trimmed.chars().last() {
        Some(c) if c.is_ascii_alphabetic() => {
            let n = trimmed.len() - c.len_utf8();
            let (num, suf) = trimmed.split_at(n);
            (num.trim(), suf.to_ascii_uppercase())
        }
        _ => (trimmed, String::new()),
    };
    if !suffix.is_empty() && suffix != "N" && suffix != "S" && suffix != "E" && suffix != "W" {
        return Err(bad());
    }
    if is_lat && (suffix == "E" || suffix == "W") {
        return Err(GisError::InvalidInput(
            "latitude takes N/S, not E/W".to_string(),
        ));
    }
    if !is_lat && (suffix == "N" || suffix == "S") {
        return Err(GisError::InvalidInput(
            "longitude takes E/W, not N/S".to_string(),
        ));
    }
    let value: f64 = num_part.parse().map_err(|_| bad())?;
    if !value.is_finite() {
        return Err(bad());
    }
    let explicit_neg = value < 0.0;
    let magnitude = value.abs();
    let negative = match suffix.as_str() {
        "S" | "W" => {
            if explicit_neg {
                return Err(GisError::InvalidInput(
                    "sign and cardinal suffix contradict".to_string(),
                ));
            }
            true
        }
        "N" | "E" => {
            if explicit_neg {
                return Err(GisError::InvalidInput(
                    "sign and cardinal suffix contradict".to_string(),
                ));
            }
            false
        }
        _ => explicit_neg,
    };
    Ok(if negative { -magnitude } else { magnitude })
}

/// Haversine distance in meters. Pure math, infallible for fenced inputs.
pub fn haversine_m(a: LonLat, b: LonLat) -> f64 {
    let r = 6_371_008.8_f64;
    let d_lat = (b.lat - a.lat).to_radians();
    let d_lng = (b.lng - a.lng).to_radians();
    let s = (d_lat / 2.0).sin().powi(2)
        + a.lat.to_radians().cos() * b.lat.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * r * s.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vienna_to_graz_is_about_145km() {
        let v = LonLat::new(16.3738, 48.2082).unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        let g = LonLat::new(15.4395, 47.0707).unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        let d = haversine_m(v, g);
        assert!(d > 130_000.0 && d < 170_000.0, "got {d}");
    }

    #[test]
    fn hostile_bbox_rejected() {
        assert!(Bbox::new(16.0, 48.0, 15.0, 49.0).is_err());
    }

    #[test]
    fn latlng_accepts_common_forms() {
        let p = parse_latlng("48.2082, 16.3738").unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        assert!((p.lat - 48.2082).abs() < 1e-9);
        assert!((p.lng - 16.3738).abs() < 1e-9);
        let s = parse_latlng("48.2082 16.3738").unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        assert!((s.lat - 48.2082).abs() < 1e-9);
        let card = parse_latlng("48.2082N, 16.3738E").unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        assert!((card.lat - 48.2082).abs() < 1e-9);
        let south = parse_latlng("33.8S, 151.2E").unwrap_or(LonLat { lng: 0.0, lat: 0.0 });
        assert!(south.lat < 0.0 && south.lng > 0.0);
    }

    #[test]
    fn latlng_rejects_garbage() {
        assert!(parse_latlng("").is_err());
        assert!(parse_latlng("48.2").is_err());
        assert!(parse_latlng("48.2, 16.3, 5.0").is_err());
        assert!(parse_latlng("abc, 16.3").is_err());
        assert!(parse_latlng("91.0, 16.3").is_err());
        assert!(parse_latlng("48.2, 191.0").is_err());
        assert!(parse_latlng("48.2E, 16.3").is_err());
        assert!(parse_latlng("-48.2S, 16.3").is_err());
        assert!(parse_latlng("48.2, NaN").is_err());
    }
}
