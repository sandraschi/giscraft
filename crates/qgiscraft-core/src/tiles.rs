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

fn cache_dir() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push("qgiscraft-tiles");
    p
}

/// Fetch one tile PNG (cache-first). Returns raw PNG bytes.
pub fn fetch_tile_png(tile: Tile) -> Result<Vec<u8>, GisError> {
    let mut path = cache_dir();
    path.push(format!("{}_{}_{}.png", tile.z, tile.x, tile.y));
    if let Ok(bytes) = std::fs::read(&path) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    let url = format!(
        "https://tile.openstreetmap.org/{}/{}/{}.png",
        tile.z, tile.x, tile.y
    );
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
}
