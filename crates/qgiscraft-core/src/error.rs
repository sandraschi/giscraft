//! Typed GIS errors. No panics, no unwraps: every failure is a value.
use std::fmt;

/// All recoverable GIS failures. Each variant carries context for logs
/// and maps to a JSON-RPC error string at the L2 boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum GisError {
    /// Remote or tile backend did not answer in time.
    Timeout(String),
    /// Bytes on the wire did not match the protocol (WMS, Overpass, GeoJSON).
    Protocol(String),
    /// Data source is known but currently unreachable.
    Offline(String),
    /// Caller-supplied argument failed validation (hostile input fence).
    InvalidInput(String),
    /// Credentials missing or rejected. Never carries secret values.
    Auth(String),
    /// Valid request this build cannot serve yet (typed, not a panic).
    Unsupported(String),
}

impl fmt::Display for GisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GisError::Timeout(ctx) => write!(f, "gis timed out: {ctx}"),
            GisError::Protocol(ctx) => write!(f, "protocol error: {ctx}"),
            GisError::Offline(ctx) => write!(f, "source offline: {ctx}"),
            GisError::InvalidInput(ctx) => write!(f, "invalid input: {ctx}"),
            GisError::Auth(ctx) => write!(f, "auth failed: {ctx}"),
            GisError::Unsupported(ctx) => write!(f, "unsupported: {ctx}"),
        }
    }
}

impl std::error::Error for GisError {}

/// Fence for every external lon/lat pair.
pub fn require_lon_lat(lng: f64, lat: f64) -> Result<(f64, f64), GisError> {
    if !lng.is_finite() || !lat.is_finite() {
        return Err(GisError::InvalidInput("lng/lat must be finite".to_string()));
    }
    if !(-180.0..=180.0).contains(&lng) {
        return Err(GisError::InvalidInput(format!(
            "lng must be -180..180, got {lng}"
        )));
    }
    if !(-90.0..=90.0).contains(&lat) {
        return Err(GisError::InvalidInput(format!(
            "lat must be -90..90, got {lat}"
        )));
    }
    Ok((lng, lat))
}

/// Fence for zoom levels.
pub fn require_zoom(zoom: u8) -> Result<u8, GisError> {
    if zoom <= 24 {
        Ok(zoom)
    } else {
        Err(GisError::InvalidInput(format!(
            "zoom must be 0..24, got {zoom}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_out_of_range_coords_rejected() {
        assert!(require_lon_lat(200.0, 48.2).is_err());
        assert!(require_lon_lat(16.3, 100.0).is_err());
        assert!(require_lon_lat(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn display_carries_context() {
        let e = GisError::Timeout("overpass".to_string());
        assert!(e.to_string().contains("overpass"));
    }
}
