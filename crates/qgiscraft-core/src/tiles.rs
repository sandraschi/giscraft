//! XYZ raster tile underlay (spike): slippy math, disk cache, PNG decode.
//! Standard OSM scheme, keyless. Cache lives in the OS temp dir for now
//! (proper `dirs` migration later). Fetch volume is small (fit bbox only)
//! with a fixed User-Agent; OSM tile policy requires attribution, which the
//! canvas draws. Network failures are typed errors, never panics.
use crate::error::GisError;

/// Tile coordinates at integer zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    pub z: u8,
    pub x: i64,
    pub y: i64,
}

/// Longitude/latitude to fractional slippy tile at zoom (Web Mercator).
/// Pure math, offline. Latitude clamped to Mercator limits.
pub fn lnglat_to_tile_float(lng: f64, lat: f64, zoom: u8) -> (f64, f64) {
    let lat = lat.clamp(-85.05112878, 85.05112878);
    let n = 2_f64.powi(zoom as i32);
    let x = (lng + 180.0) / 360.0 * n;
    let lat_rad = lat.to_radians();
    let y = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// Tile range covering a lng/lat bbox at integer zoom.
pub fn bbox_to_tiles(
    min_lng: f64,
    min_lat: f64,
    max_lng: f64,
    max_lat: f64,
    zoom: u8,
) -> Result<Vec<Tile>, GisError> {
    if min_lng > max_lng || min_lat > max_lat {
        return Err(GisError::InvalidInput(
            "bbox min must be <= max".to_string(),
        ));
    }
    let zoom = zoom.min(19);
    let (x0, y0) = lnglat_to_tile_float(min_lng, max_lat, zoom);
    let (x1, y1) = lnglat_to_tile_float(max_lng, min_lat, zoom);
    let n = 2_i64.pow(zoom as u32);
    let xa = (x0.floor() as i64).clamp(0, n - 1);
    let xb = (x1.floor() as i64).clamp(0, n - 1);
    let ya = (y0.floor() as i64).clamp(0, n - 1);
    let yb = (y1.floor() as i64).clamp(0, n - 1);
    let count = (xb - xa + 1) * (yb - ya + 1);
    if count > 64 {
        return Err(GisError::InvalidInput(format!(
            "tile range too large ({count} tiles, cap 64): zoom out or shrink bbox"
        )));
    }
    let mut out = Vec::new();
    for x in xa..=xb {
        for y in ya..=yb {
            out.push(Tile { z: zoom, x, y });
        }
    }
    Ok(out)
}

/// Tile geographic bounds (min_lng, min_lat, max_lng, max_lat).
pub fn tile_bounds(tile: Tile) -> (f64, f64, f64, f64) {
    let n = 2_f64.powi(tile.z as i32);
    let min_lng = tile.x as f64 / n * 360.0 - 180.0;
    let max_lng = (tile.x + 1) as f64 / n * 360.0 - 180.0;
    let inv = |y: f64| {
        let t = std::f64::consts::PI * (1.0 - 2.0 * y / n);
        t.sinh().atan().to_degrees()
    };
    let max_lat = inv(tile.y as f64);
    let min_lat = inv(tile.y as f64 + 1.0);
    (min_lng, min_lat, max_lng, max_lat)
}

/// Tile pixel (px, py, 0..256, origin top-left) to lng/lat.
pub fn tile_pixel_to_lnglat(tile: Tile, px: f64, py: f64) -> (f64, f64) {
    let n = 2_f64.powi(tile.z as i32);
    let lng = (tile.x as f64 + px / 256.0) / n * 360.0 - 180.0;
    let t = std::f64::consts::PI * (1.0 - 2.0 * (tile.y as f64 + py / 256.0) / n);
    (lng, t.sinh().atan().to_degrees())
}

/// World size in pixels at a float slippy zoom. Shared by the native canvas
/// and the headless previews so pixels agree everywhere.
pub fn world_px(zoom: f32) -> f64 {
    256.0 * 2_f64.powf(zoom as f64)
}

/// lng/lat -> screen pixels. `(cx, cy)` is the view center in screen px
/// (usually the canvas middle). Pure Web Mercator, no I/O.
pub fn project(
    lng: f64,
    lat: f64,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    cx: f32,
    cy: f32,
) -> (f32, f32) {
    let world = world_px(zoom);
    let xw = world * (lng + 180.0) / 360.0;
    let yw = world
        * (1.0
            - (lat.to_radians().tan() + 1.0 / lat.to_radians().cos()).ln() / std::f64::consts::PI)
        / 2.0;
    let cxw = world * (center_lng + 180.0) / 360.0;
    let cyw = world
        * (1.0
            - (center_lat.to_radians().tan() + 1.0 / center_lat.to_radians().cos()).ln()
                / std::f64::consts::PI)
        / 2.0;
    (cx + (xw - cxw) as f32, cy + (yw - cyw) as f32)
}

/// Screen pixels -> lng/lat. Inverse of [`project`].
pub fn unproject(
    x: f32,
    y: f32,
    center_lng: f64,
    center_lat: f64,
    zoom: f32,
    cx: f32,
    cy: f32,
) -> (f64, f64) {
    let world = world_px(zoom);
    let cxw = world * (center_lng + 180.0) / 360.0;
    let cyw = world
        * (1.0
            - (center_lat.to_radians().tan() + 1.0 / center_lat.to_radians().cos()).ln()
                / std::f64::consts::PI)
        / 2.0;
    let xw = cxw + (x - cx) as f64;
    let yw = cyw + (y - cy) as f64;
    let lng = xw / world * 360.0 - 180.0;
    let t = std::f64::consts::PI * (1.0 - 2.0 * yw / world);
    (lng, t.sinh().atan().to_degrees())
}

/// Fit a lng/lat bbox into a WxH view with 20% padding. Returns
/// `(center_lng, center_lat, zoom)`. None when the bbox is empty.
/// World01 math (never degrees), clamped to slippy zoom 2..19.
pub fn fit_view(
    min_lng: f64,
    min_lat: f64,
    max_lng: f64,
    max_lat: f64,
    width: f32,
    height: f32,
) -> Option<(f64, f64, f32)> {
    if !min_lng.is_finite() || !max_lng.is_finite() || !min_lat.is_finite() || !max_lat.is_finite()
    {
        return None;
    }
    if min_lng > max_lng || min_lat > max_lat {
        return None;
    }
    let to_x = |lng: f64| (lng + 180.0) / 360.0;
    let to_y = |lat: f64| {
        let r = lat.clamp(-85.05112878, 85.05112878).to_radians();
        (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0
    };
    let (x0, x1) = (to_x(min_lng), to_x(max_lng));
    let (mut y0, mut y1) = (to_y(max_lat), to_y(min_lat));
    if y0 > y1 {
        std::mem::swap(&mut y0, &mut y1);
    }
    let cx = (x0 + x1) / 2.0;
    let cy = (y0 + y1) / 2.0;
    let span_x = ((x1 - x0) * 1.2).max(1e-9);
    let span_y = ((y1 - y0) * 1.2).max(1e-9);
    let zoom = (width / (span_x * 256.0) as f32)
        .min(height / (span_y * 256.0) as f32)
        .log2()
        .clamp(2.0, 19.0);
    let from_x = |x: f64| x * 360.0 - 180.0;
    let t = std::f64::consts::PI * (1.0 - 2.0 * cy);
    Some((from_x(cx), t.sinh().atan().to_degrees(), zoom))
}

/// New view center so that geo (glng, glat) stays pinned at screen point
/// (ax, ay) when the canvas center is (cx, cy). Zoom-to-cursor and pan are
/// both this: pick the geo to pin, then recenter on it.
pub fn recenter(glng: f64, glat: f64, ax: f32, ay: f32, cx: f32, cy: f32, zoom: f32) -> (f64, f64) {
    let world = world_px(zoom);
    let r = glat.clamp(-85.05112878, 85.05112878).to_radians();
    let xw = world * (glng + 180.0) / 360.0 - (ax - cx) as f64;
    let yw = world * (1.0 - (r.tan() + 1.0 / r.cos()).ln() / std::f64::consts::PI) / 2.0
        - (ay - cy) as f64;
    let lng = xw / world * 360.0 - 180.0;
    let t = std::f64::consts::PI * (1.0 - 2.0 * yw / world);
    (lng, t.sinh().atan().to_degrees())
}

/// Hit-testing shared by the native canvas and any future picker.
/// All pure math, degrees in / pixels out, no I/O.
/// Even-odd point-in-ring test (lon/lat space is fine at city scale).
/// Edges count as inside (a click exactly on a border selects the polygon).
pub fn point_in_ring(lng: f64, lat: f64, ring: &[(f64, f64)]) -> bool {
    if !lng.is_finite() || !lat.is_finite() || ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let n = ring.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = match ring.get(i) {
            Some(&p) => p,
            None => {
                j = i;
                continue;
            }
        };
        let (xj, yj) = match ring.get(j) {
            Some(&p) => p,
            None => {
                j = i;
                continue;
            }
        };
        if dist_to_segment_deg(lng, lat, xi, yi, xj, yj) < 1e-12 {
            return true;
        }
        if (yi > lat) != (yj > lat) {
            let x_int = xi + (lat - yi) * (xj - xi) / (yj - yi);
            if lng < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}
/// Distance in degrees from point p to segment a-b. Used for line picking
/// (the caller converts a pixel tolerance into degrees with the local
/// meters-per-pixel scale).
pub fn dist_to_segment_deg(
    plng: f64,
    plat: f64,
    alng: f64,
    alat: f64,
    blng: f64,
    blat: f64,
) -> f64 {
    let dx = blng - alng;
    let dy = blat - alat;
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0 || !len2.is_finite() {
        let ddx = plng - alng;
        let ddy = plat - alat;
        return (ddx * ddx + ddy * ddy).sqrt();
    }
    let t = (((plng - alng) * dx + (plat - alat) * dy) / len2).clamp(0.0, 1.0);
    let cx = alng + t * dx;
    let cy = alat + t * dy;
    let ddx = plng - cx;
    let ddy = plat - cy;
    (ddx * ddx + ddy * ddy).sqrt()
}

pub fn meters_per_pixel(lat: f64, zoom: f32) -> f64 {
    156543.03392 * lat.to_radians().cos() / 2_f64.powf(zoom as f64)
}

/// Nice round bar length (1/2/5 x 10^n meters) that fits `max_px` pixels.
pub fn nice_scale_bar(mpp: f64, max_px: f32) -> (f64, f64) {
    if mpp <= 0.0 || max_px <= 0.0 || !mpp.is_finite() || !max_px.is_finite() {
        return (0.0, 0.0);
    }
    let raw = mpp * max_px as f64;
    if !raw.is_finite() || raw <= 0.0 {
        return (0.0, 0.0);
    }
    let pow10 = 10_f64.powf(raw.log10().floor());
    for m in [5.0, 2.0, 1.0] {
        let candidate = m * pow10;
        if candidate <= raw {
            return (candidate, candidate / mpp);
        }
    }
    (pow10 / 10.0, pow10 / 10.0 / mpp)
}

/// Deterministic per-layer color from a fixed high-contrast palette./// Same id always maps to the same color across native canvas, headless
/// BMP, and web preview (app wraps this into `egui::Color32`).
pub fn layer_color(id: &str) -> (u8, u8, u8) {
    const PALETTE: [[u8; 3]; 8] = [
        [0x4c, 0xc9, 0xf0],
        [0xff, 0xb0, 0x26],
        [0x80, 0xed, 0x99],
        [0xff, 0x70, 0xa6],
        [0xff, 0xe0, 0x66],
        [0xc7, 0x7d, 0xff],
        [0xff, 0x59, 0x64],
        [0x64, 0xdf, 0xdf],
    ];
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let c = PALETTE[(h as usize) % PALETTE.len()];
    (c[0], c[1], c[2])
}

/// Parse a `#rrggbb` display color. Empty means "palette default" and
/// parses to `None`; anything else malformed is a typed error.
pub fn parse_layer_color(text: &str) -> Result<Option<(u8, u8, u8)>, crate::error::GisError> {
    use crate::error::GisError;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let bytes = trimmed.as_bytes();
    let hex_ok =
        bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|b| b.is_ascii_hexdigit());
    if !hex_ok {
        return Err(GisError::InvalidInput(
            "color must be #rrggbb or empty".to_string(),
        ));
    }
    let hex = |i: usize| {
        let s = &trimmed[i..i + 2];
        u8::from_str_radix(s, 16)
            .map_err(|_| GisError::InvalidInput("color must be #rrggbb or empty".to_string()))
    };
    Ok(Some((hex(1)?, hex(3)?, hex(5)?)))
}

/// XYZ tile URL template (`{z}/{x}/{y}` placeholders).
/// Override with `QGISCRAFT_TILES_URL` (e.g. a local mirror or style).
/// No network here: pure string handling, so tests stay offline.
pub fn tile_url_template() -> String {
    std::env::var("QGISCRAFT_TILES_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string())
}

/// Expand a `{z}/{x}/{y}` template for one tile. Pure string work.
pub fn expand_template(template: &str, tile: Tile) -> String {
    template
        .replace("{z}", &tile.z.to_string())
        .replace("{x}", &tile.x.to_string())
        .replace("{y}", &tile.y.to_string())
}

/// Expand the configured template for one tile.
pub fn tile_url(tile: Tile) -> String {
    expand_template(&tile_url_template(), tile)
}

/// A template is usable when it has all three placeholders and an
/// http(s) URL around them. Raster layers with anything else (legacy
/// stubs like `xyz:osm`) are ignored by the renderer instead of
/// shadowing the default basemap with a broken URL.
pub fn is_usable_template(template: &str) -> bool {
    let t = template.trim();
    (t.starts_with("https://") || t.starts_with("http://"))
        && t.contains("{z}")
        && t.contains("{x}")
        && t.contains("{y}")
}

/// Short hash to namespace the disk cache per tile template.
/// Public so the app can key its texture cache the same way.
pub fn template_key(template: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in template.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// Fetch one tile PNG with an explicit template (cache-first, namespaced).
/// `fetch_tile_png` is this with the configured default template.
pub fn fetch_tile_png_as(tile: Tile, template: &str) -> Result<Vec<u8>, GisError> {
    let mut path = std::env::temp_dir();
    path.push("qgiscraft-tiles");
    path.push(template_key(template));
    path.push(format!("{}_{}_{}.png", tile.z, tile.x, tile.y));
    if let Ok(bytes) = std::fs::read(&path) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    let url = expand_template(template, tile);
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(GisError::InvalidInput(
            "tile URL must start with http(s)://".to_string(),
        ));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("qgiscraft-mcp/0.1 (dev spike, few tiles; OSM attribution shown)")
        .build()
        .map_err(|e| GisError::Offline(format!("http client: {e}")))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| GisError::Offline(format!("tile unreachable: {e}")))?;
    if !resp.status().is_success() {
        return Err(GisError::Offline(format!(
            "tile status {}",
            resp.status().as_u16()
        )));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| GisError::Protocol(format!("tile body: {e}")))?
        .to_vec();
    if bytes.is_empty() {
        return Err(GisError::Protocol("tile empty body".to_string()));
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &bytes);
    Ok(bytes)
}

/// Fetch one tile PNG (cache-first) with the configured default template.
pub fn fetch_tile_png(tile: Tile) -> Result<Vec<u8>, GisError> {
    fetch_tile_png_as(tile, &tile_url_template())
}

/// Decode PNG bytes to flat RGBA8 (row-major, top-down).
pub fn decode_png_rgba(png: &[u8]) -> Result<(u32, u32, Vec<u8>), GisError> {
    if png.is_empty() {
        return Err(GisError::InvalidInput("png is empty".to_string()));
    }
    let img = image::load_from_memory(png)
        .map_err(|e| GisError::Protocol(format!("png decode: {e}")))?
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || w > 1024 || h > 1024 {
        return Err(GisError::Protocol(format!(
            "tile dims out of range: {w}x{h}"
        )));
    }
    Ok((w, h, img.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearby_points_share_tile_distant_ones_do_not() {
        let (x0, y0) = lnglat_to_tile_float(16.3738, 48.2082, 16);
        let (x1, y1) = lnglat_to_tile_float(16.3740, 48.2084, 16);
        assert_eq!(x0.floor() as i64, x1.floor() as i64);
        assert_eq!(y0.floor() as i64, y1.floor() as i64);
        // Graz is far from Stephansplatz: different tile.
        let (gx, gy) = lnglat_to_tile_float(15.4395, 47.0707, 16);
        assert!(gx.floor() as i64 != x0.floor() as i64 || gy.floor() as i64 != y0.floor() as i64);
    }

    #[test]
    fn innere_stadt_bbox_is_small_at_z15() {
        let tiles = bbox_to_tiles(16.36, 48.20, 16.39, 48.22, 15);
        assert!(tiles.is_ok());
        if let Ok(tiles) = tiles {
            assert!(!tiles.is_empty() && tiles.len() <= 64);
        }
    }

    #[test]
    fn hostile_bbox_and_huge_range_rejected_offline() {
        assert!(bbox_to_tiles(16.4, 48.2, 16.3, 48.3, 15).is_err());
        // Whole world at z10 = way over the 64-tile cap.
        assert!(bbox_to_tiles(-180.0, -85.0, 180.0, 85.0, 10).is_err());
    }

    #[test]
    fn empty_png_rejected_without_network() {
        assert!(decode_png_rgba(&[]).is_err());
    }

    #[test]
    fn template_expands_placeholders() {
        let url = expand_template(
            "https://tiles.example.org/{z}/{x}/{y}.png",
            Tile {
                z: 15,
                x: 17688,
                y: 11054,
            },
        );
        assert_eq!(url, "https://tiles.example.org/15/17688/11054.png");
    }

    #[test]
    fn template_is_nonempty() {
        assert!(!tile_url_template().trim().is_empty());
    }

    #[test]
    fn unusable_templates_rejected() {
        assert!(is_usable_template(
            "https://tile.openstreetmap.org/{z}/{x}/{y}.png"
        ));
        assert!(!is_usable_template("xyz:osm"));
        assert!(!is_usable_template("https://tiles.example.org/{z}/{x}.png"));
        assert!(!is_usable_template(""));
        assert!(!is_usable_template(
            "ftp://tiles.example.org/{z}/{x}/{y}.png"
        ));
    }

    #[test]
    fn tile_bounds_contain_center_pixel() {
        let tile = Tile {
            z: 16,
            x: 100,
            y: 200,
        };
        let (min_lng, min_lat, max_lng, max_lat) = tile_bounds(tile);
        assert!(min_lng < max_lng && min_lat < max_lat);
        let (clng, clat) = tile_pixel_to_lnglat(tile, 128.0, 128.0);
        assert!(clng > min_lng && clng < max_lng);
        assert!(clat > min_lat && clat < max_lat);
    }

    #[test]
    fn project_centers_and_doubles_with_zoom() {
        let (x0, y0) = project(16.3738, 48.2082, 16.3738, 48.2082, 15.0, 400.0, 300.0);
        assert!((x0 - 400.0).abs() < 0.01 && (y0 - 300.0).abs() < 0.01);
        let (xa, _) = project(16.3838, 48.2082, 16.3738, 48.2082, 15.0, 400.0, 300.0);
        let (xb, _) = project(16.3838, 48.2082, 16.3738, 48.2082, 16.0, 400.0, 300.0);
        let ratio = (xb - 400.0) / (xa - 400.0);
        assert!((ratio - 2.0).abs() < 0.01, "zoom+1 must double: {ratio}");
    }

    #[test]
    fn unproject_inverts_project() {
        let (lng, lat) = unproject(450.0, 280.0, 16.3738, 48.2082, 14.0, 400.0, 300.0);
        let (x, y) = project(lng, lat, 16.3738, 48.2082, 14.0, 400.0, 300.0);
        assert!((x - 450.0).abs() < 0.01 && (y - 280.0).abs() < 0.01);
    }

    #[test]
    fn fit_view_frames_vienna_bbox() {
        let fit = fit_view(16.36, 48.20, 16.39, 48.22, 800.0, 600.0);
        assert!(fit.is_some());
        if let Some((clng, clat, zoom)) = fit {
            assert!((clng - 16.375).abs() < 1e-6, "clng {clng}");
            assert!((clat - 48.21).abs() < 1e-3, "clat {clat}");
            assert!((12.0..=17.0).contains(&zoom), "zoom {zoom}");
        }
        assert!(fit_view(16.4, 48.2, 16.3, 48.3, 800.0, 600.0).is_none());
        assert!(fit_view(f64::NAN, 0.0, 1.0, 1.0, 800.0, 600.0).is_none());
    }

    #[test]
    fn recenter_keeps_anchor_fixed() {
        // Pin Stephansplatz at an off-center pixel, then verify it projects
        // back onto that same pixel with the new center.
        let (glng, glat) = (16.3738, 48.2082);
        let (ax, ay, cx, cy, zoom) = (500.0_f32, 200.0, 400.0, 300.0, 15.0);
        let (nclng, nclat) = recenter(glng, glat, ax, ay, cx, cy, zoom);
        let (px, py) = project(glng, glat, nclng, nclat, zoom, cx, cy);
        assert!((px - ax).abs() < 0.05 && (py - ay).abs() < 0.05);
    }

    #[test]
    fn vienna_scale_is_sane() {
        // ~3.2 m/px at Stephansplatz, z15.
        let mpp = meters_per_pixel(48.2082, 15.0);
        assert!((3.0..3.5).contains(&mpp), "mpp {mpp}");
        // Nice bars snap to 1/2/5 and fit the pixel budget.
        let (m, px) = nice_scale_bar(mpp, 120.0);
        assert!([100.0, 200.0, 500.0].contains(&m), "bar {m}m");
        assert!(px <= 120.0 && px > 0.0);
        assert_eq!(nice_scale_bar(0.0, 100.0), (0.0, 0.0));
        assert_eq!(nice_scale_bar(f64::NAN, 100.0), (0.0, 0.0));
    }

    #[test]
    fn picking_point_in_ring_and_segments() {
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!(point_in_ring(0.5, 0.5, &square));
        assert!(!point_in_ring(2.0, 0.5, &square));
        assert!(point_in_ring(0.0, 0.5, &square));
        assert!(!point_in_ring(0.5, 0.5, &[]));
        assert!(!point_in_ring(f64::NAN, 0.5, &square));
        // C-shape: bay is outside.
        let c = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 1.0),
            (1.0, 1.0),
            (1.0, 3.0),
            (4.0, 3.0),
            (4.0, 4.0),
            (0.0, 4.0),
        ];
        assert!(point_in_ring(0.5, 2.0, &c));
        assert!(!point_in_ring(2.5, 2.0, &c));
        // Segment distance: on-line ~0, off-line grows, endpoints clamp.
        assert!(dist_to_segment_deg(0.5, 0.0, 0.0, 0.0, 1.0, 0.0) < 1e-12);
        assert!((dist_to_segment_deg(0.5, 1.0, 0.0, 0.0, 1.0, 0.0) - 1.0).abs() < 1e-9);
        assert!(dist_to_segment_deg(2.0, 0.0, 0.0, 0.0, 1.0, 0.0) > 0.9);
        assert!(dist_to_segment_deg(5.0, 5.0, 5.0, 5.0, 5.0, 5.0) < 1e-12);
    }

    #[test]
    fn layer_color_hex_parses_and_rejects() {
        assert_eq!(parse_layer_color("").unwrap_or(Some((9, 9, 9))), None);
        assert_eq!(
            parse_layer_color("#ff0000").unwrap_or(None),
            Some((255, 0, 0))
        );
        assert_eq!(
            parse_layer_color("  #00FF00 ").unwrap_or(None),
            Some((0, 255, 0))
        );
        assert!(parse_layer_color("red").is_err());
        assert!(parse_layer_color("#fff").is_err());
        assert!(parse_layer_color("#gggggg").is_err());
        assert!(parse_layer_color("#ff00000").is_err());
    }
}
