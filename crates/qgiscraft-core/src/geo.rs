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
}
