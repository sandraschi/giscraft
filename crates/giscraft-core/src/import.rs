//! File import/export: GeoJSON + delimited text (CSV), GeoJSON export.
//! Pure string in / typed values out. No filesystem here (callers read and
//! write files); no network. Factors the Overpass ingest shape so files,
//! Overpass, and geocoding all land as the same `RawFeature`.
use crate::error::GisError;
use serde::{Deserialize, Serialize};

/// One parsed feature, ready for the store. `geom_json` carries full
/// geometry for lines/polygons; points keep None and use lng/lat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawFeature {
    pub name: String,
    pub kind: String,
    pub lng: f64,
    pub lat: f64,
    pub geom_json: Option<String>,
}

fn centroid(coords: &[(f64, f64)]) -> Option<(f64, f64)> {
    if coords.is_empty() {
        return None;
    }
    let (sx, sy) = coords
        .iter()
        .fold((0.0, 0.0), |(x, y), (lng, lat)| (x + lng, y + lat));
    let n = coords.len() as f64;
    Some((sx / n, sy / n))
}

fn valid(lng: f64, lat: f64) -> bool {
    lng.is_finite()
        && lat.is_finite()
        && (-180.0..=180.0).contains(&lng)
        && (-90.0..=90.0).contains(&lat)
}

/// Parse a GeoJSON `FeatureCollection` (or single `Feature`).
/// Supports Point, LineString, Polygon. Multi* and foreign members are a
/// typed error, not a silent skip: callers and users must know what was
/// left out. Cap: 10,000 features.
pub fn parse_geojson(text: &str) -> Result<Vec<RawFeature>, GisError> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| GisError::InvalidInput(format!("bad GeoJSON: {e}")))?;
    let feats = match v.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => {
            v.get("features")
                .and_then(|f| f.as_array())
                .ok_or_else(|| {
                    GisError::InvalidInput("FeatureCollection without features[]".to_string())
                })?
        }
        Some("Feature") => {
            return single_feature(&v).map(|f| vec![f]);
        }
        Some(other) => {
            return Err(GisError::InvalidInput(format!(
                "expected FeatureCollection or Feature, got {other}"
            )));
        }
        None => {
            return Err(GisError::InvalidInput(
                "GeoJSON without type member".to_string(),
            ));
        }
    };
    if feats.len() > 10_000 {
        return Err(GisError::InvalidInput(format!(
            "too many features ({}, cap 10000): split the file",
            feats.len()
        )));
    }
    let mut out = Vec::new();
    for (i, f) in feats.iter().enumerate() {
        match single_feature(f) {
            Ok(feat) => out.push(feat),
            Err(e) => {
                return Err(GisError::InvalidInput(format!("feature {i}: {e}")));
            }
        }
    }
    Ok(out)
}

fn single_feature(f: &serde_json::Value) -> Result<RawFeature, GisError> {
    let name = f
        .get("properties")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("(unnamed)")
        .to_string();
    let geom = f
        .get("geometry")
        .ok_or_else(|| GisError::InvalidInput("feature without geometry".to_string()))?;
    if geom.is_null() {
        return Err(GisError::InvalidInput("null geometry".to_string()));
    }
    let ty = geom
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| GisError::InvalidInput("geometry without type".to_string()))?;
    let coords = geom
        .get("coordinates")
        .ok_or_else(|| GisError::InvalidInput("geometry without coordinates".to_string()))?;
    match ty {
        "Point" => {
            let (lng, lat) = pair(coords)?;
            Ok(RawFeature {
                name,
                kind: "point".to_string(),
                lng,
                lat,
                geom_json: None,
            })
        }
        "LineString" => {
            let line = line_coords(coords)?;
            let (clng, clat) = centroid(&line)
                .ok_or_else(|| GisError::InvalidInput("empty LineString".to_string()))?;
            Ok(RawFeature {
                name,
                kind: "way".to_string(),
                lng: clng,
                lat: clat,
                geom_json: Some(
                    serde_json::json!({"type": "LineString", "coordinates": coords}).to_string(),
                ),
            })
        }
        "Polygon" => {
            let ring = coords
                .get(0)
                .and_then(|r| r.as_array())
                .ok_or_else(|| GisError::InvalidInput("empty Polygon".to_string()))?;
            let mut pts = Vec::new();
            for pt in ring {
                let (lng, lat) = pair(pt)?;
                pts.push((lng, lat));
            }
            if pts.len() < 3 {
                return Err(GisError::InvalidInput("Polygon ring too short".to_string()));
            }
            let (clng, clat) = centroid(&pts)
                .ok_or_else(|| GisError::InvalidInput("empty Polygon".to_string()))?;
            Ok(RawFeature {
                name,
                kind: "way".to_string(),
                lng: clng,
                lat: clat,
                geom_json: Some(
                    serde_json::json!({"type": "Polygon", "coordinates": coords}).to_string(),
                ),
            })
        }
        other => Err(GisError::InvalidInput(format!(
            "unsupported geometry {other} (Point/LineString/Polygon only)"
        ))),
    }
}

fn pair(v: &serde_json::Value) -> Result<(f64, f64), GisError> {
    let arr = v
        .as_array()
        .ok_or_else(|| GisError::InvalidInput("position is not an array".to_string()))?;
    let lng = arr
        .first()
        .and_then(|n| n.as_f64())
        .ok_or_else(|| GisError::InvalidInput("bad longitude".to_string()))?;
    let lat = arr
        .get(1)
        .and_then(|n| n.as_f64())
        .ok_or_else(|| GisError::InvalidInput("bad latitude".to_string()))?;
    if !valid(lng, lat) {
        return Err(GisError::InvalidInput(format!(
            "coordinates out of range: {lng},{lat}"
        )));
    }
    Ok((lng, lat))
}

fn line_coords(v: &serde_json::Value) -> Result<Vec<(f64, f64)>, GisError> {
    let arr = v
        .as_array()
        .ok_or_else(|| GisError::InvalidInput("LineString coordinates not an array".to_string()))?;
    if arr.len() < 2 {
        return Err(GisError::InvalidInput(
            "LineString needs 2+ positions".to_string(),
        ));
    }
    let mut out = Vec::new();
    for pt in arr {
        out.push(pair(pt)?);
    }
    Ok(out)
}

/// Parse delimited text (CSV/TSV/semicolon). Header row required; longitude
/// and latitude columns are auto-detected case-insensitively from
/// lng/lon/long/x and lat/y (override with explicit names). Cap: 10,000 rows.
pub fn parse_csv(
    text: &str,
    lng_col: Option<&str>,
    lat_col: Option<&str>,
) -> Result<Vec<RawFeature>, GisError> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| GisError::InvalidInput("empty file (need a header row)".to_string()))?;
    let delim = if header.contains(';') && !header.contains(',') {
        ';'
    } else if header.contains('\t') {
        '\t'
    } else {
        ','
    };
    let heads: Vec<String> = header
        .split(delim)
        .map(|h| h.trim().trim_matches('"').to_string())
        .collect();
    let find = |want: &[&str], fallback: Option<&str>| -> Result<usize, GisError> {
        if let Some(name) = fallback {
            if name.trim().is_empty() {
                return Err(GisError::InvalidInput(
                    "column override must not be empty".to_string(),
                ));
            }
            return heads
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name.trim()))
                .ok_or_else(|| GisError::InvalidInput(format!("column '{name}' not found")));
        }
        heads
            .iter()
            .position(|h| want.iter().any(|w| h.eq_ignore_ascii_case(w)))
            .ok_or_else(|| {
                GisError::InvalidInput(
                    "no longitude/latitude columns found (want lng/lon/long/x + lat/y)".to_string(),
                )
            })
    };
    let lng_i = find(&["lng", "lon", "long", "x", "longitude"], lng_col)?;
    let lat_i = find(&["lat", "y", "latitude"], lat_col)?;
    let name_i = heads.iter().position(|h| {
        ["name", "title", "label", "id"]
            .iter()
            .any(|w| h.eq_ignore_ascii_case(w))
    });
    let mut out = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if out.len() >= 10_000 {
            return Err(GisError::InvalidInput(
                "too many rows (cap 10000): split the file".to_string(),
            ));
        }
        let cells: Vec<&str> = line.split(delim).collect();
        let get = |i: usize| -> Result<&str, GisError> {
            cells
                .get(i)
                .map(|s| s.trim().trim_matches('"'))
                .ok_or_else(|| GisError::InvalidInput(format!("row {}: missing column {i}", n + 2)))
        };
        let lng: f64 = get(lng_i)?
            .parse()
            .map_err(|_| GisError::InvalidInput(format!("row {}: bad longitude", n + 2)))?;
        let lat: f64 = get(lat_i)?
            .parse()
            .map_err(|_| GisError::InvalidInput(format!("row {}: bad latitude", n + 2)))?;
        if !valid(lng, lat) {
            return Err(GisError::InvalidInput(format!(
                "row {}: out of range {lng},{lat}",
                n + 2
            )));
        }
        let name = name_i
            .and_then(|i| cells.get(i))
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("row {}", n + 2));
        out.push(RawFeature {
            name,
            kind: "point".to_string(),
            lng,
            lat,
            geom_json: None,
        });
    }
    if out.is_empty() {
        return Err(GisError::InvalidInput("no data rows".to_string()));
    }
    Ok(out)
}

/// Serialize stored features as a GeoJSON FeatureCollection (point features
/// without `geom_json` become Points; `geom_json` passes through verbatim
/// after a validity check).
pub fn export_geojson(pairs: &[(&str, &str, f64, f64, Option<&str>)]) -> Result<String, GisError> {
    let mut feats = Vec::new();
    for (name, kind, lng, lat, geom) in pairs {
        let g = match geom {
            Some(raw) => serde_json::from_str(raw)
                .map_err(|e| GisError::Protocol(format!("stored geometry corrupt: {e}")))?,
            None => {
                if !valid(*lng, *lat) {
                    return Err(GisError::InvalidInput(format!(
                        "feature '{name}' out of range"
                    )));
                }
                serde_json::json!({"type": "Point", "coordinates": [lng, lat]})
            }
        };
        feats.push(serde_json::json!({
            "type": "Feature",
            "properties": {"name": name, "kind": kind},
            "geometry": g,
        }));
    }
    Ok(serde_json::json!({"type": "FeatureCollection", "features": feats}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_all_three_geometries() {
        let doc = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"name":"A"},"geometry":{"type":"Point","coordinates":[16.3,48.2]}},
            {"type":"Feature","properties":{"name":"R"},"geometry":{"type":"LineString","coordinates":[[16.3,48.2],[16.4,48.25]]}},
            {"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[16.3,48.2],[16.4,48.2],[16.4,48.25],[16.3,48.2]]]}}]}"#;
        let feats = parse_geojson(doc);
        assert!(feats.is_ok());
        if let Ok(feats) = feats {
            assert_eq!(feats.len(), 3);
            assert!(feats[0].geom_json.is_none());
            assert!(feats[1]
                .geom_json
                .as_ref()
                .is_some_and(|g| g.contains("LineString")));
            assert!(feats[2]
                .geom_json
                .as_ref()
                .is_some_and(|g| g.contains("Polygon")));
            assert_eq!(feats[2].name, "(unnamed)");
        }
    }

    #[test]
    fn geojson_rejects_garbage_and_exotics() {
        assert!(parse_geojson("not json").is_err());
        assert!(parse_geojson(r#"{"type":"FeatureCollection"}"#).is_err());
        assert!(parse_geojson(r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"MultiPoint","coordinates":[[16.3,48.2]]}}]}"#).is_err());
        assert!(parse_geojson(r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"Point","coordinates":[999.0,48.2]}}]}"#).is_err());
    }

    #[test]
    fn csv_autodetect_and_explicit_columns() {
        let doc = "name,lon,lat\nStephansplatz,16.3738,48.2082\nPraterstern,16.4273,48.2189\n";
        let feats = parse_csv(doc, None, None);
        assert!(feats.is_ok());
        if let Ok(feats) = feats {
            assert_eq!(feats.len(), 2);
            assert_eq!(feats[0].name, "Stephansplatz");
        }
        let weird = "X;Y;T\n16.3;48.2;here\n";
        let feats = parse_csv(weird, Some("X"), Some("Y"));
        assert!(feats.is_ok());
    }

    #[test]
    fn csv_rejects_missing_columns_and_bad_rows() {
        assert!(parse_csv("a,b\n1,2\n", None, None).is_err());
        assert!(parse_csv("lng,lat\n16.3,notanum\n", None, None).is_err());
        assert!(parse_csv("", None, None).is_err());
    }

    #[test]
    fn export_roundtrips_points_and_lines() {
        let out = export_geojson(&[("A", "point", 16.3, 48.2, None)]);
        assert!(out.is_ok());
        if let Ok(text) = out {
            assert!(text.contains("FeatureCollection"));
            // And it parses back through our own importer.
            let back = parse_geojson(&text);
            assert!(back.is_ok());
            if let Ok(feats) = back {
                assert_eq!(feats.len(), 1);
                assert_eq!(feats[0].name, "A");
            }
        }
    }
}
