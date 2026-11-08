//! Minimal WKT reader: POINT / LINESTRING / POLYGON and MULTI* (2D only).
//! No deps, no I/O. Higher dimensions, junk around the geometry, and
//! unbalanced structure are typed errors. Empty geometries parse to zero
//! features. Polygon rings are auto-closed when the writer left them open.
use crate::error::GisError;
use crate::geo::LonLat;

const MAX_COORDS: usize = 100_000;

/// One parsed geometry: representative center + GeoJSON for the store.
#[derive(Debug, Clone, PartialEq)]
pub struct WktFeature {
    pub name: String,
    pub kind: String,
    pub lng: f64,
    pub lat: f64,
    pub geojson: String,
}

struct Cursor {
    chars: Vec<char>,
    pos: usize,
    coords: usize,
}

impl Cursor {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
            coords: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.bump();
        }
    }

    fn expect(&mut self, want: char, what: &str) -> Result<(), GisError> {
        self.skip_spaces();
        match self.peek() {
            Some(c) if c == want => {
                self.bump();
                Ok(())
            }
            _ => Err(GisError::InvalidInput(format!("bad WKT: expected {what}"))),
        }
    }

    fn number(&mut self) -> Result<f64, GisError> {
        self.skip_spaces();
        let mut text = String::new();
        if matches!(self.peek(), Some('-' | '+')) {
            if let Some(c) = self.peek() {
                text.push(c);
                self.bump();
            }
        }
        let mut digits = 0;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                digits += 1;
                text.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some('.')) {
            text.push('.');
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    digits += 1;
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
        }
        if digits == 0 {
            return Err(GisError::InvalidInput(
                "bad WKT: expected number".to_string(),
            ));
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            text.push('e');
            self.bump();
            if matches!(self.peek(), Some('-' | '+')) {
                if let Some(c) = self.peek() {
                    text.push(c);
                    self.bump();
                }
            }
            let mut exp_digits = 0;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    exp_digits += 1;
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            if exp_digits == 0 {
                return Err(GisError::InvalidInput("bad WKT: bad exponent".to_string()));
            }
        }
        text.parse::<f64>()
            .map_err(|_| GisError::InvalidInput("bad WKT: bad number".to_string()))
            .and_then(|v| {
                if v.is_finite() {
                    Ok(v)
                } else {
                    Err(GisError::InvalidInput(
                        "bad WKT: number not finite".to_string(),
                    ))
                }
            })
    }

    /// One 2D pair. A third number means 3D input, which we reject.
    fn pair(&mut self) -> Result<(f64, f64), GisError> {
        let x = self.number()?;
        let y = self.number()?;
        self.skip_spaces();
        if let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
                return Err(GisError::InvalidInput(
                    "bad WKT: 3D coordinates not supported (2D only)".to_string(),
                ));
            }
        }
        self.coords = self.coords.saturating_add(1);
        if self.coords > MAX_COORDS {
            return Err(GisError::InvalidInput(
                "bad WKT: too many coordinates".to_string(),
            ));
        }
        Ok((x, y))
    }

    fn ring(&mut self) -> Result<Vec<(f64, f64)>, GisError> {
        self.expect('(', "'('")?;
        let mut pts = Vec::new();
        loop {
            self.skip_spaces();
            if self.peek() == Some(')') {
                self.bump();
                break;
            }
            pts.push(self.pair()?);
            self.skip_spaces();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(')') => {
                    self.bump();
                    break;
                }
                _ => {
                    return Err(GisError::InvalidInput(
                        "bad WKT: expected ',' or ')'".to_string(),
                    ));
                }
            }
        }
        if pts.is_empty() {
            return Err(GisError::InvalidInput("bad WKT: empty ring".to_string()));
        }
        Ok(pts)
    }
}

fn fmt_num(v: f64) -> String {
    format!("{v}")
}

fn point_geojson(x: f64, y: f64) -> String {
    format!(
        "{{\"type\":\"Point\",\"coordinates\":[{},{}]}}",
        fmt_num(x),
        fmt_num(y)
    )
}

fn line_geojson(pts: &[(f64, f64)]) -> String {
    let inner: Vec<String> = pts
        .iter()
        .map(|(x, y)| format!("[{},{}]", fmt_num(*x), fmt_num(*y)))
        .collect();
    format!(
        "{{\"type\":\"LineString\",\"coordinates\":[{}]}}",
        inner.join(",")
    )
}

fn close_ring(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = pts.to_vec();
    let first = pts.first().copied();
    let last = pts.last().copied();
    if first != last {
        if let Some(f) = first {
            out.push(f);
        }
        let _ = last;
    }
    out
}

fn poly_geojson(rings: &[Vec<(f64, f64)>]) -> String {
    let parts: Vec<String> = rings
        .iter()
        .map(|ring| {
            let closed = close_ring(ring);
            let inner: Vec<String> = closed
                .iter()
                .map(|(x, y)| format!("[{},{}]", fmt_num(*x), fmt_num(*y)))
                .collect();
            format!("[{}]", inner.join(","))
        })
        .collect();
    format!(
        "{{\"type\":\"Polygon\",\"coordinates\":[{}]}}",
        parts.join(",")
    )
}

fn center_of(pts: &[(f64, f64)]) -> (f64, f64) {
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut n = 0_usize;
    for (x, y) in pts {
        sx += *x;
        sy += *y;
        n = n.saturating_add(1);
    }
    if n == 0 {
        return (0.0, 0.0);
    }
    (sx / n as f64, sy / n as f64)
}

fn check_point(x: f64, y: f64) -> Result<LonLat, GisError> {
    LonLat::new(x, y)
        .map_err(|_| GisError::InvalidInput(format!("bad WKT: coordinate out of range ({x}, {y})")))
}

/// Parse WKT text into features. Names are `wkt-1`, `wkt-2`, ...
pub fn parse_wkt(text: &str) -> Result<Vec<WktFeature>, GisError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(GisError::InvalidInput("wkt must not be empty".to_string()));
    }
    if trimmed.chars().count() > 1_000_000 {
        return Err(GisError::InvalidInput(
            "wkt exceeds 1000000 chars".to_string(),
        ));
    }
    let mut cursor = Cursor::new(trimmed);
    // Optional leading SRID tag.
    let mut probe = Cursor::new(trimmed);
    let mut tag = String::new();
    while let Some(c) = probe.peek() {
        if c.is_ascii_alphabetic() || c == '=' {
            tag.push(c);
            probe.bump();
        } else {
            break;
        }
    }
    if tag.eq_ignore_ascii_case("SRID=") {
        let mut digits = 0;
        while let Some(c) = probe.peek() {
            if c.is_ascii_digit() {
                digits += 1;
                probe.bump();
            } else {
                break;
            }
        }
        if digits == 0 || probe.peek() != Some(';') {
            return Err(GisError::InvalidInput("bad WKT: bad SRID tag".to_string()));
        }
        probe.bump();
        cursor.pos = probe.pos;
    } else if trimmed.contains(';') {
        return Err(GisError::InvalidInput(
            "bad WKT: unexpected ';'".to_string(),
        ));
    }
    cursor.skip_spaces();
    let mut keyword = String::new();
    while let Some(c) = cursor.peek() {
        if c.is_ascii_alphabetic() {
            keyword.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    let keyword = keyword.to_ascii_uppercase();
    let supported = [
        "POINT",
        "LINESTRING",
        "POLYGON",
        "MULTIPOINT",
        "MULTILINESTRING",
        "MULTIPOLYGON",
    ];
    if !supported.contains(&keyword.as_str()) {
        return Err(GisError::InvalidInput(format!(
            "bad WKT: unsupported type '{keyword}'"
        )));
    }
    cursor.skip_spaces();
    // EMPTY variant parses to zero features.
    let mut word = String::new();
    let save = cursor.pos;
    while let Some(c) = cursor.peek() {
        if c.is_ascii_alphabetic() {
            word.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    if word.eq_ignore_ascii_case("EMPTY") {
        cursor.skip_spaces();
        if cursor.peek().is_some() {
            return Err(GisError::InvalidInput(
                "bad WKT: trailing input".to_string(),
            ));
        }
        return Ok(Vec::new());
    }
    cursor.pos = save;
    let mut features: Vec<WktFeature> = Vec::new();
    let mut index = 0_usize;
    let mut push = |kind: &str, lng: f64, lat: f64, geojson: String| {
        index = index.saturating_add(1);
        features.push(WktFeature {
            name: format!("wkt-{index}"),
            kind: kind.to_string(),
            lng,
            lat,
            geojson,
        });
    };
    match keyword.as_str() {
        "POINT" => {
            cursor.expect('(', "'('")?;
            let (x, y) = cursor.pair()?;
            cursor.skip_spaces();
            cursor.expect(')', "')'")?;
            check_point(x, y)?;
            push("point", x, y, point_geojson(x, y));
        }
        "LINESTRING" => {
            let pts = cursor.ring()?;
            if pts.len() < 2 {
                return Err(GisError::InvalidInput(
                    "bad WKT: LineString needs 2+ points".to_string(),
                ));
            }
            for (x, y) in &pts {
                check_point(*x, *y)?;
            }
            let (cx, cy) = center_of(&pts);
            push("way", cx, cy, line_geojson(&pts));
        }
        "POLYGON" => {
            cursor.expect('(', "'('")?;
            let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
            loop {
                cursor.skip_spaces();
                if cursor.peek() == Some(')') {
                    cursor.bump();
                    break;
                }
                rings.push(cursor.ring()?);
                cursor.skip_spaces();
                match cursor.peek() {
                    Some(',') => {
                        cursor.bump();
                    }
                    Some(')') => {
                        cursor.bump();
                        break;
                    }
                    _ => {
                        return Err(GisError::InvalidInput(
                            "bad WKT: expected ',' or ')'".to_string(),
                        ));
                    }
                }
                if rings.is_empty() {
                    return Err(GisError::InvalidInput("bad WKT: empty Polygon".to_string()));
                }
            }
            if rings.is_empty() {
                return Err(GisError::InvalidInput("bad WKT: empty Polygon".to_string()));
            }
            let mut all: Vec<(f64, f64)> = Vec::new();
            for ring in &rings {
                if ring.len() < 3 {
                    return Err(GisError::InvalidInput(
                        "bad WKT: ring needs 3+ points".to_string(),
                    ));
                }
                for (x, y) in ring {
                    check_point(*x, *y)?;
                    all.push((*x, *y));
                }
            }
            let (cx, cy) = center_of(&all);
            push("way", cx, cy, poly_geojson(&rings));
        }
        "MULTIPOINT" => {
            cursor.expect('(', "'('")?;
            cursor.skip_spaces();
            if cursor.peek() == Some('(') {
                // Parenthesized form: ((x y), (x y)).
                loop {
                    cursor.skip_spaces();
                    if cursor.peek() == Some(')') {
                        cursor.bump();
                        break;
                    }
                    cursor.expect('(', "'('")?;
                    let (x, y) = cursor.pair()?;
                    cursor.skip_spaces();
                    cursor.expect(')', "')'")?;
                    check_point(x, y)?;
                    push("point", x, y, point_geojson(x, y));
                    cursor.skip_spaces();
                    match cursor.peek() {
                        Some(',') => {
                            cursor.bump();
                        }
                        Some(')') => {
                            cursor.bump();
                            break;
                        }
                        _ => {
                            return Err(GisError::InvalidInput(
                                "bad WKT: expected ',' or ')'".to_string(),
                            ));
                        }
                    }
                }
            } else {
                // Flat form: (x y, x y).
                loop {
                    cursor.skip_spaces();
                    if cursor.peek() == Some(')') {
                        cursor.bump();
                        break;
                    }
                    let (x, y) = cursor.pair()?;
                    check_point(x, y)?;
                    push("point", x, y, point_geojson(x, y));
                    cursor.skip_spaces();
                    match cursor.peek() {
                        Some(',') => {
                            cursor.bump();
                        }
                        Some(')') => {
                            cursor.bump();
                            break;
                        }
                        _ => {
                            return Err(GisError::InvalidInput(
                                "bad WKT: expected ',' or ')'".to_string(),
                            ));
                        }
                    }
                }
            }
            cursor.skip_spaces();
            if cursor.peek().is_some() {
                return Err(GisError::InvalidInput(
                    "bad WKT: trailing input".to_string(),
                ));
            }
            return Ok(features);
        }
        "MULTILINESTRING" => {
            cursor.expect('(', "'('")?;
            loop {
                cursor.skip_spaces();
                if cursor.peek() == Some(')') {
                    cursor.bump();
                    break;
                }
                let pts = cursor.ring()?;
                if pts.len() < 2 {
                    return Err(GisError::InvalidInput(
                        "bad WKT: LineString needs 2+ points".to_string(),
                    ));
                }
                for (x, y) in &pts {
                    check_point(*x, *y)?;
                }
                let (cx, cy) = center_of(&pts);
                push("way", cx, cy, line_geojson(&pts));
                cursor.skip_spaces();
                match cursor.peek() {
                    Some(',') => {
                        cursor.bump();
                    }
                    Some(')') => {
                        cursor.bump();
                        break;
                    }
                    _ => {
                        return Err(GisError::InvalidInput(
                            "bad WKT: expected ',' or ')'".to_string(),
                        ));
                    }
                }
            }
        }
        "MULTIPOLYGON" => {
            cursor.expect('(', "'('")?;
            loop {
                cursor.skip_spaces();
                if cursor.peek() == Some(')') {
                    cursor.bump();
                    break;
                }
                cursor.expect('(', "'('")?;
                let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
                loop {
                    cursor.skip_spaces();
                    if cursor.peek() == Some(')') {
                        cursor.bump();
                        break;
                    }
                    rings.push(cursor.ring()?);
                    cursor.skip_spaces();
                    match cursor.peek() {
                        Some(',') => {
                            cursor.bump();
                        }
                        Some(')') => {
                            cursor.bump();
                            break;
                        }
                        _ => {
                            return Err(GisError::InvalidInput(
                                "bad WKT: expected ',' or ')'".to_string(),
                            ));
                        }
                    }
                }
                if rings.is_empty() {
                    return Err(GisError::InvalidInput("bad WKT: empty Polygon".to_string()));
                }
                let mut all: Vec<(f64, f64)> = Vec::new();
                for ring in &rings {
                    if ring.len() < 3 {
                        return Err(GisError::InvalidInput(
                            "bad WKT: ring needs 3+ points".to_string(),
                        ));
                    }
                    for (x, y) in ring {
                        check_point(*x, *y)?;
                        all.push((*x, *y));
                    }
                }
                let (cx, cy) = center_of(&all);
                push("way", cx, cy, poly_geojson(&rings));
                cursor.skip_spaces();
                match cursor.peek() {
                    Some(',') => {
                        cursor.bump();
                    }
                    Some(')') => {
                        cursor.bump();
                        break;
                    }
                    _ => {
                        return Err(GisError::InvalidInput(
                            "bad WKT: expected ',' or ')'".to_string(),
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(GisError::InvalidInput(format!(
                "bad WKT: unsupported type '{keyword}'"
            )));
        }
    }
    cursor.skip_spaces();
    if cursor.peek().is_some() {
        return Err(GisError::InvalidInput(
            "bad WKT: trailing input".to_string(),
        ));
    }
    if features.is_empty() {
        return Err(GisError::InvalidInput("bad WKT: no geometries".to_string()));
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_parses() {
        let feats = parse_wkt("POINT (16.3738 48.2082)").unwrap_or_default();
        assert_eq!(feats.len(), 1);
        if let Some(f) = feats.first() {
            assert_eq!(f.kind, "point");
            assert!((f.lng - 16.3738).abs() < 1e-9);
            assert!(f.geojson.contains("Point"));
        }
    }

    #[test]
    fn lowercase_and_srid_accepted() {
        let feats = parse_wkt("srid=4326;point(16.3 48.2)").unwrap_or_default();
        assert_eq!(feats.len(), 1);
    }

    #[test]
    fn linestring_and_polygon_parse() {
        let line = parse_wkt("LINESTRING (16.3 48.2, 16.4 48.25)").unwrap_or_default();
        assert_eq!(line.len(), 1);
        if let Some(f) = line.first() {
            assert_eq!(f.kind, "way");
            assert!(f.geojson.contains("LineString"));
        }
        let poly =
            parse_wkt("POLYGON ((16.3 48.2, 16.4 48.2, 16.4 48.3, 16.3 48.2))").unwrap_or_default();
        assert_eq!(poly.len(), 1);
        if let Some(f) = poly.first() {
            assert!(f.geojson.contains("Polygon"));
        }
    }

    #[test]
    fn multi_forms_parse() {
        let flat = parse_wkt("MULTIPOINT (16.3 48.2, 16.4 48.25)").unwrap_or_default();
        assert_eq!(flat.len(), 2);
        let paren = parse_wkt("MULTIPOINT ((16.3 48.2), (16.4 48.25))").unwrap_or_default();
        assert_eq!(paren.len(), 2);
        let mls = parse_wkt("MULTILINESTRING ((16.3 48.2, 16.4 48.25), (16.5 48.2, 16.6 48.3))")
            .unwrap_or_default();
        assert_eq!(mls.len(), 2);
    }

    #[test]
    fn empty_and_unclosed_ring() {
        assert!(parse_wkt("POINT EMPTY").unwrap_or_default().is_empty());
        let open = parse_wkt("POLYGON ((16.3 48.2, 16.4 48.2, 16.4 48.3))").unwrap_or_default();
        assert_eq!(open.len(), 1);
        if let Some(f) = open.first() {
            // Auto-closed: first == last coordinate pair.
            assert!(f.geojson.ends_with("[16.3,48.2]]]}"));
        }
    }

    #[test]
    fn hostile_inputs_rejected() {
        assert!(parse_wkt("").is_err());
        assert!(parse_wkt("POINT (16.3)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2, 5.0)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2 5.0)").is_err());
        assert!(parse_wkt("POINT (NaN 48.2)").is_err());
        assert!(parse_wkt("POINT (16.3 948.2)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2)").is_ok());
        assert!(parse_wkt("CIRCLE (16.3 48.2)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2))").is_err());
        assert!(parse_wkt("POINT ((16.3 48.2)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2) trailing").is_err());
        assert!(parse_wkt("LINESTRING (16.3 48.2)").is_err());
        assert!(parse_wkt("POINT (16.3 48.2; 5.0)").is_err());
        assert!(parse_wkt("SRID=abc;POINT (16.3 48.2)").is_err());
    }
}
