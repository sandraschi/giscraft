//! Doppler geolocation (colsto method, offline MVP): match inaudible
//! propeller-plane Doppler curves from a recording against ADS-B tracks.
//! For each grid candidate the forward model predicts the observed pitch
//! curve (sound-travel delay iterated twice, MSL minus terrain for range)
//! and scores it by normalized correlation against the observed curve.
//! No network, no audio decoding: the caller supplies curves + tracks.
//! Multi-curve captures score the mean of per-curve best-track matches.
//! The 920s-gap negative-evidence filter from the video is NOT in v1
//! (documented in docs/SPEC-doppler-locate.md): v1 ranks by shape only.
use crate::error::GisError;
use serde::{Deserialize, Serialize};

pub const SOUND_SPEED_MS: f64 = 343.0;
const MAX_GRID_CELLS: usize = 10_000;
const MAX_TOP_N: usize = 100;

/// One ADS-B position report. `t` is seconds relative to the clip start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackSample {
    pub t: f64,
    pub lng: f64,
    pub lat: f64,
    pub alt_m: f64,
    pub speed_ms: f64,
    pub heading_deg: f64,
}

/// One observed Doppler curve: base frequency + (time, observed Hz) pairs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DopplerCurve {
    pub f0: f64,
    pub samples: Vec<(f64, f64)>,
}

/// A capture file: candidate tracks plus observed curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DopplerCapture {
    pub tracks: Vec<Vec<TrackSample>>,
    pub curves: Vec<DopplerCurve>,
}

/// One ranked candidate:LngLat centroid + mean correlation score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredPoint {
    pub lng: f64,
    pub lat: f64,
    pub score: f64,
}

fn meters_per_deg_lng(lat: f64) -> f64 {
    111_320.0 * lat.to_radians().cos().abs().max(0.01)
}

/// Linearly interpolate track position/speed at `t`. No extrapolation.
fn interp_pos(track: &[TrackSample], t: f64) -> Result<(f64, f64, f64, f64, f64), GisError> {
    if track.len() < 2 {
        return Err(GisError::InvalidInput("track needs 2+ samples".to_string()));
    }
    let first = &track[0];
    let last = &track[track.len() - 1];
    if !t.is_finite() || t < first.t || t > last.t {
        return Err(GisError::InvalidInput(
            "time outside track range".to_string(),
        ));
    }
    for w in track.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t >= a.t && t <= b.t {
            let span = b.t - a.t;
            let k = if span <= 0.0 { 0.0 } else { (t - a.t) / span };
            let lng = a.lng + (b.lng - a.lng) * k;
            let lat = a.lat + (b.lat - a.lat) * k;
            let alt = a.alt_m + (b.alt_m - a.alt_m) * k;
            let speed = a.speed_ms + (b.speed_ms - a.speed_ms) * k;
            let nearer = if (t - a.t) <= (b.t - t) { a } else { b };
            return Ok((lng, lat, alt, speed, nearer.heading_deg));
        }
    }
    Err(GisError::InvalidInput(
        "time outside track range".to_string(),
    ))
}

/// Predicted observed frequency at candidate (clng, clat), time `t_obs`.
fn predict(
    track: &[TrackSample],
    clng: f64,
    clat: f64,
    t_obs: f64,
    f0: f64,
    terrain_m: f64,
) -> Result<f64, GisError> {
    let mpd_lng = meters_per_deg_lng(clat);
    let mut t_em = t_obs;
    let mut dx = 0.0;
    let mut dy = 0.0;
    let mut r = 0.0;
    for _ in 0..2 {
        // Clamp: emission before the first (or after the last) report uses
        // the endpoint position. Documented approximation for short tracks.
        let tq = t_em.clamp(track[0].t, track[track.len() - 1].t);
        let (lng, lat, alt, _, _) = interp_pos(track, tq)?;
        dx = (clng - lng) * mpd_lng;
        dy = (clat - lat) * 111_320.0;
        let dz = terrain_m - alt;
        r = (dx * dx + dy * dy + dz * dz).sqrt();
        if !r.is_finite() || r <= 0.0 {
            return Err(GisError::InvalidInput("degenerate range".to_string()));
        }
        t_em = t_obs - r / SOUND_SPEED_MS;
    }
    let (_, _, _, speed, heading) =
        interp_pos(track, t_em.clamp(track[0].t, track[track.len() - 1].t))?;
    let brng = heading.to_radians();
    let v_east = speed * brng.sin();
    let v_north = speed * brng.cos();
    let v_radial = (v_east * dx + v_north * dy) / r;
    let denom = SOUND_SPEED_MS - v_radial;
    if !denom.is_finite() || denom <= 1.0 {
        return Err(GisError::InvalidInput("supersonic geometry".to_string()));
    }
    Ok(f0 * SOUND_SPEED_MS / denom)
}

fn normalized(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let var = xs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / xs.len() as f64;
    let std = var.sqrt();
    if !std.is_finite() || std <= 0.0 {
        return vec![0.0; xs.len()];
    }
    xs.iter().map(|x| (x - mean) / std).collect()
}

fn correlation(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot / a.len() as f64
}

/// Score one curve against one track at a candidate point.
fn score_curve(
    track: &[TrackSample],
    curve: &DopplerCurve,
    clng: f64,
    clat: f64,
    terrain_m: f64,
) -> Result<f64, GisError> {
    if !curve.f0.is_finite() || curve.f0 <= 0.0 {
        return Err(GisError::InvalidInput(
            "curve f0 must be positive".to_string(),
        ));
    }
    if curve.samples.len() < 3 {
        return Err(GisError::InvalidInput("curve needs 3+ samples".to_string()));
    }
    let mut pred = Vec::new();
    let mut obs = Vec::new();
    for (t, f) in &curve.samples {
        if !t.is_finite() || !f.is_finite() || *f <= 0.0 {
            return Err(GisError::InvalidInput(
                "curve samples must be finite positive".to_string(),
            ));
        }
        pred.push(predict(track, clng, clat, *t, curve.f0, terrain_m)?);
        obs.push(*f);
    }
    Ok(correlation(&normalized(&pred), &normalized(&obs)))
}

/// Parse + validate a capture document.
pub fn parse_capture(text: &str) -> Result<DopplerCapture, GisError> {
    if text.trim().is_empty() {
        return Err(GisError::InvalidInput(
            "capture must not be empty".to_string(),
        ));
    }
    if text.chars().count() > 5_000_000 {
        return Err(GisError::InvalidInput(
            "capture exceeds 5000000 chars".to_string(),
        ));
    }
    let cap: DopplerCapture = serde_json::from_str(text)
        .map_err(|e| GisError::InvalidInput(format!("capture parse: {e}")))?;
    if cap.tracks.is_empty() || cap.curves.is_empty() {
        return Err(GisError::InvalidInput(
            "capture needs tracks and curves".to_string(),
        ));
    }
    if cap.tracks.len() > 64 || cap.curves.len() > 16 {
        return Err(GisError::InvalidInput(
            "capture too large (64 tracks / 16 curves max)".to_string(),
        ));
    }
    for (i, track) in cap.tracks.iter().enumerate() {
        if track.len() < 2 || track.len() > 100_000 {
            return Err(GisError::InvalidInput(format!(
                "track {i} needs 2..100000 samples"
            )));
        }
        for s in track {
            crate::geo::LonLat::new(s.lng, s.lat).map_err(|_| {
                GisError::InvalidInput(format!("track {i} coordinate out of range"))
            })?;
            for v in [s.t, s.alt_m, s.speed_ms, s.heading_deg] {
                if !v.is_finite() {
                    return Err(GisError::InvalidInput(format!(
                        "track {i} has non-finite field"
                    )));
                }
            }
            if s.speed_ms < 0.0 || s.speed_ms > 400.0 {
                return Err(GisError::InvalidInput(format!(
                    "track {i} speed out of range"
                )));
            }
        }
    }
    Ok(cap)
}

/// Rank parameters: bbox, grid step, result cap, ground elevation.
pub struct LocateParams {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
    pub step_deg: f64,
    pub top_n: usize,
    pub terrain_m: f64,
}

/// Rank grid candidates over a bbox. Score = mean over curves of the best
/// per-track correlation. Returns the top `top_n` (1..100).
pub fn locate(params: &LocateParams, cap: &DopplerCapture) -> Result<Vec<ScoredPoint>, GisError> {
    let bbox = crate::geo::Bbox::new(params.west, params.south, params.east, params.north)?;
    if !params.step_deg.is_finite() || params.step_deg < 0.0005 || params.step_deg > 1.0 {
        return Err(GisError::InvalidInput(
            "step_deg must be 0.0005..1.0".to_string(),
        ));
    }
    if !params.terrain_m.is_finite() || params.terrain_m < -500.0 || params.terrain_m > 9000.0 {
        return Err(GisError::InvalidInput("terrain_m out of range".to_string()));
    }
    let top_n = params.top_n.clamp(1, MAX_TOP_N);
    let rows = ((bbox.max_lat - bbox.min_lat) / params.step_deg).ceil() as usize;
    let cols = ((bbox.max_lng - bbox.min_lng) / params.step_deg).ceil() as usize;
    let cells = rows
        .checked_mul(cols)
        .ok_or_else(|| GisError::InvalidInput("grid too large".to_string()))?;
    if cells == 0 || cells > MAX_GRID_CELLS {
        return Err(GisError::InvalidInput(format!(
            "grid would build {cells} cells (cap {MAX_GRID_CELLS})"
        )));
    }
    let mut scored: Vec<ScoredPoint> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let clng = bbox.min_lng + (c as f64 + 0.5) * params.step_deg;
            let clat = bbox.min_lat + (r as f64 + 0.5) * params.step_deg;
            let mut total = 0.0;
            for curve in &cap.curves {
                let mut best = f64::NEG_INFINITY;
                for track in &cap.tracks {
                    match score_curve(track, curve, clng, clat, params.terrain_m) {
                        Ok(s) => {
                            if s > best {
                                best = s;
                            }
                        }
                        Err(_) => continue,
                    }
                }
                if best == f64::NEG_INFINITY {
                    best = -1.0;
                }
                total += best;
            }
            scored.push(ScoredPoint {
                lng: clng,
                lat: clat,
                score: total / cap.curves.len() as f64,
            });
        }
    }
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_n);
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(south: f64, west: f64, north: f64, east: f64, step_deg: f64) -> LocateParams {
        LocateParams {
            south,
            west,
            north,
            east,
            step_deg,
            top_n: 5,
            terrain_m: 200.0,
        }
    }

    fn straight_track() -> Vec<TrackSample> {
        // Eastbound prop at 60 m/s, 500 m MSL, one sample/sec for 60 s.
        (0..60)
            .map(|i| TrackSample {
                t: i as f64,
                lng: 16.30 + i as f64 * 0.0008,
                lat: 48.20,
                alt_m: 500.0,
                speed_ms: 60.0,
                heading_deg: 90.0,
            })
            .collect()
    }

    fn curve_from_truth(truth_lng: f64, truth_lat: f64, f0: f64) -> DopplerCurve {
        let track = straight_track();
        let mut samples = Vec::new();
        let mut t = 5.0;
        while t < 55.0 {
            if let Ok(f) = predict(&track, truth_lng, truth_lat, t, f0, 200.0) {
                samples.push((t, f));
            }
            t += 2.0;
        }
        DopplerCurve { f0, samples }
    }

    #[test]
    fn forward_model_falls_through_closest_approach() {
        let track = straight_track();
        let before = predict(&track, 16.33, 48.21, 5.0, 100.0, 200.0);
        let after = predict(&track, 16.33, 48.21, 55.0, 100.0, 200.0);
        assert!(before.is_ok() && after.is_ok());
        if let (Ok(b), Ok(a)) = (before, after) {
            // Approaching: pitch up; receding: pitch down.
            assert!(b > 100.0, "before {b}");
            assert!(a < 100.0, "after {a}");
        }
    }

    #[test]
    fn locate_recovers_synthetic_truth() {
        let track = straight_track();
        let curve = curve_from_truth(16.33, 48.21, 120.0);
        assert!(curve.samples.len() >= 3);
        let cap = DopplerCapture {
            tracks: vec![track],
            curves: vec![curve],
        };
        let out = locate(&params(48.19, 16.31, 48.23, 16.35, 0.005), &cap);
        assert!(out.is_ok());
        if let Ok(out) = out {
            assert_eq!(out.len(), 5);
            let top = &out[0];
            let d_lng = (top.lng - 16.33).abs();
            let d_lat = (top.lat - 48.21).abs();
            assert!(d_lng < 0.01 && d_lat < 0.01, "top {top:?}");
            assert!(top.score > 0.9, "score {}", top.score);
        }
    }

    #[test]
    fn hostile_captures_rejected() {
        assert!(parse_capture("").is_err());
        assert!(parse_capture("{nope").is_err());
        let empty = DopplerCapture {
            tracks: vec![],
            curves: vec![],
        };
        assert!(serde_json::to_string(&empty).is_ok());
        let bad = DopplerCapture {
            tracks: vec![],
            curves: vec![DopplerCurve {
                f0: 100.0,
                samples: vec![(0.0, 100.0)],
            }],
        };
        let text = serde_json::to_string(&bad).unwrap_or_default();
        assert!(parse_capture(&text).is_err());
        let track = straight_track();
        // Negative f0: every curve errors, all candidates score -1 (no match).
        let cap_neg = DopplerCapture {
            tracks: vec![track],
            curves: vec![DopplerCurve {
                f0: -5.0,
                samples: vec![(0.0, 100.0), (1.0, 101.0), (2.0, 102.0)],
            }],
        };
        let out_neg = locate(&params(48.19, 16.31, 48.23, 16.35, 0.01), &cap_neg);
        assert!(out_neg.is_ok());
        if let Ok(out) = out_neg {
            assert!(out.iter().all(|p| p.score < 0.0));
        }
        // Reversed bbox and absurd step rejected.
        let cap_good = DopplerCapture {
            tracks: vec![straight_track()],
            curves: vec![curve_from_truth(16.33, 48.21, 120.0)],
        };
        assert!(locate(&params(48.23, 16.31, 48.19, 16.35, 0.01), &cap_good).is_err());
        assert!(locate(&params(48.19, 16.31, 48.23, 16.35, 5.0), &cap_good).is_err());
    }
}
