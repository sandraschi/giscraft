//! L7 bearing lens: position + compass heading ray-cast against OSM building footprints
//! and cultural venue polygons.
use serde::{Deserialize, Serialize};
use serde_json::Value;

const EARTH_R_M: f64 = 6_371_000.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingFootprint {
    pub name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikidata_qid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikipedia: Option<String>,
    /// Closed or open polygon ring in `(lat, lon)`.
    pub polygon: Vec<(f64, f64)>,
    pub center_lat: f64,
    pub center_lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BearingHit {
    pub name: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikidata_qid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikipedia: Option<String>,
    pub ray_distance_m: f64,
    pub bearing_deg: f64,
    pub angle_diff_deg: f64,
    pub direct_ray_hit: bool,
}

/// Convert `(lat, lon)` to local ENU meters `(east_x, north_y)` relative to `(origin_lat, origin_lon)`.
fn to_local_m(origin_lat: f64, origin_lon: f64, lat: f64, lon: f64) -> (f64, f64) {
    let lat_rad = origin_lat.to_radians();
    let x = (lon - origin_lon).to_radians() * EARTH_R_M * lat_rad.cos();
    let y = (lat - origin_lat).to_radians() * EARTH_R_M;
    (x, y)
}

/// Compass bearing in degrees `[0, 360)` from `(lat1, lon1)` to `(lat2, lon2)`.
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (x, y) = to_local_m(lat1, lon1, lat2, lon2);
    let deg = x.atan2(y).to_degrees();
    (deg + 360.0) % 360.0
}

/// Smallest angular difference `[0, 180]` between two compass bearings.
pub fn angle_diff_deg(a: f64, b: f64) -> f64 {
    let d = ((a - b + 540.0) % 360.0) - 180.0;
    d.abs()
}

/// Ray-segment intersection in 2D: ray from `(0, 0)` with unit direction `(dx, dy)`
/// against segment `(ax, ay) -> (bx, by)`. Returns `Some(t)` with `t >= 0` if hit.
fn ray_segment_t(dx: f64, dy: f64, ax: f64, ay: f64, bx: f64, by: f64) -> Option<f64> {
    let sx = bx - ax;
    let sy = by - ay;
    let denom = dx * sy - dy * sx;
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (ax * sy - ay * sx) / denom;
    let u = (ax * dy - ay * dx) / denom;
    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

/// Ray-cast from `(lat, lon)` along `heading_deg` (clockwise from North) against building footprints.
/// Direct polygon ray intersections rank first by distance; buildings within `half_fov_deg`
/// of the heading are included as cone fallback matches.
pub fn ray_cast_bearing(
    lat: f64,
    lon: f64,
    heading_deg: f64,
    max_dist_m: f64,
    half_fov_deg: f64,
    buildings: &[BuildingFootprint],
) -> Vec<BearingHit> {
    let rad = heading_deg.to_radians();
    let dx = rad.sin();
    let dy = rad.cos();
    let mut hits = Vec::new();

    for b in buildings {
        let mut best_t: Option<f64> = None;
        if b.polygon.len() >= 2 {
            for win in b.polygon.windows(2) {
                let (ax, ay) = to_local_m(lat, lon, win[0].0, win[0].1);
                let (bx, by) = to_local_m(lat, lon, win[1].0, win[1].1);
                if let Some(t) = ray_segment_t(dx, dy, ax, ay, bx, by) {
                    if t <= max_dist_m {
                        best_t = Some(best_t.map_or(t, |cur| cur.min(t)));
                    }
                }
            }
            if let (Some(first), Some(last)) = (b.polygon.first(), b.polygon.last()) {
                if first != last {
                    let (ax, ay) = to_local_m(lat, lon, last.0, last.1);
                    let (bx, by) = to_local_m(lat, lon, first.0, first.1);
                    if let Some(t) = ray_segment_t(dx, dy, ax, ay, bx, by) {
                        if t <= max_dist_m {
                            best_t = Some(best_t.map_or(t, |cur| cur.min(t)));
                        }
                    }
                }
            }
        }

        let (cx, cy) = to_local_m(lat, lon, b.center_lat, b.center_lon);
        let center_dist = (cx * cx + cy * cy).sqrt();
        let b_deg = bearing_deg(lat, lon, b.center_lat, b.center_lon);
        let diff = angle_diff_deg(heading_deg, b_deg);

        if let Some(t) = best_t {
            hits.push(BearingHit {
                name: b.name.clone(),
                category: b.category.clone(),
                wikidata_qid: b.wikidata_qid.clone(),
                wikipedia: b.wikipedia.clone(),
                ray_distance_m: t,
                bearing_deg: b_deg,
                angle_diff_deg: diff,
                direct_ray_hit: true,
            });
        } else if center_dist <= max_dist_m && diff <= half_fov_deg {
            hits.push(BearingHit {
                name: b.name.clone(),
                category: b.category.clone(),
                wikidata_qid: b.wikidata_qid.clone(),
                wikipedia: b.wikipedia.clone(),
                ray_distance_m: center_dist,
                bearing_deg: b_deg,
                angle_diff_deg: diff,
                direct_ray_hit: false,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.direct_ray_hit
            .cmp(&a.direct_ray_hit)
            .then_with(|| a.ray_distance_m.total_cmp(&b.ray_distance_m))
    });
    hits
}

/// Parse Overpass JSON (`out geom;` or `out center;`) into `BuildingFootprint` entries.
pub fn parse_overpass_buildings(raw: &Value) -> Vec<BuildingFootprint> {
    let Some(elements) = raw.get("elements").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for el in elements {
        let Some(tags) = el.get("tags").and_then(Value::as_object) else {
            continue;
        };
        let Some(name) = tags.get("name").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let polygon: Vec<(f64, f64)> = el
            .get("geometry")
            .and_then(Value::as_array)
            .map(|pts| {
                pts.iter()
                    .filter_map(|p| {
                        Some((
                            p.get("lat").and_then(Value::as_f64)?,
                            p.get("lon").and_then(Value::as_f64)?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let (center_lat, center_lon) = if let (Some(la), Some(lo)) = (
            el.pointer("/center/lat")
                .or_else(|| el.get("lat"))
                .and_then(Value::as_f64),
            el.pointer("/center/lon")
                .or_else(|| el.get("lon"))
                .and_then(Value::as_f64),
        ) {
            (la, lo)
        } else if !polygon.is_empty() {
            let n = polygon.len() as f64;
            let s_lat: f64 = polygon.iter().map(|p| p.0).sum();
            let s_lon: f64 = polygon.iter().map(|p| p.1).sum();
            (s_lat / n, s_lon / n)
        } else {
            continue;
        };

        let category = tags
            .get("tourism")
            .or_else(|| tags.get("historic"))
            .or_else(|| tags.get("building"))
            .and_then(Value::as_str)
            .unwrap_or("building")
            .to_string();

        out.push(BuildingFootprint {
            name: name.to_string(),
            category,
            wikidata_qid: tags
                .get("wikidata")
                .and_then(Value::as_str)
                .map(str::to_string),
            wikipedia: tags
                .get("wikipedia")
                .and_then(Value::as_str)
                .map(str::to_string),
            polygon,
            center_lat,
            center_lon,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ray_cast_hits_building_directly_ahead() {
        // Observer at Maria-Theresien-Platz (48.2045, 16.3607), looking SE (155 deg) toward KHM vs NW (335 deg) toward NHM
        let raw = json!({
            "elements": [
                {
                    "type": "way",
                    "tags": {"name": "Kunsthistorisches Museum", "tourism": "museum", "wikidata": "Q95569"},
                    "geometry": [
                        {"lat": 48.2035, "lon": 16.3610},
                        {"lat": 48.2035, "lon": 16.3622},
                        {"lat": 48.2041, "lon": 16.3622},
                        {"lat": 48.2041, "lon": 16.3610}
                    ]
                },
                {
                    "type": "way",
                    "tags": {"name": "Naturhistorisches Museum", "tourism": "museum", "wikidata": "Q688704"},
                    "geometry": [
                        {"lat": 48.2050, "lon": 16.3594},
                        {"lat": 48.2050, "lon": 16.3606},
                        {"lat": 48.2056, "lon": 16.3606},
                        {"lat": 48.2056, "lon": 16.3594}
                    ]
                }
            ]
        });
        let buildings = parse_overpass_buildings(&raw);
        let se_hits = ray_cast_bearing(48.2045, 16.3607, 145.0, 400.0, 30.0, &buildings);
        assert_eq!(se_hits.len(), 1);
        assert_eq!(se_hits[0].name, "Kunsthistorisches Museum");
        assert!(se_hits[0].direct_ray_hit);

        let nw_hits = ray_cast_bearing(48.2045, 16.3607, 325.0, 400.0, 30.0, &buildings);
        assert_eq!(nw_hits.len(), 1);
        assert_eq!(nw_hits[0].name, "Naturhistorisches Museum");
        assert!(nw_hits[0].direct_ray_hit);
    }
}
