//! Thin HTTP sidecar (Track B seed): the same command bus over REST.
//! No new capabilities here - every handler dispatches into
//! `qgiscraft-core` / `qgiscraft-store` exactly like the MCP layer does.
//! Serves the built console from `web/dist` when present (Vite build output);
//! in dev, run Vite on :11253 with its proxy pointed here (:11254).
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

fn db_path(param: Option<String>) -> Option<String> {
    param.filter(|s| !s.trim().is_empty())
}

fn open_db(db: &Option<String>) -> Result<rusqlite::Connection, String> {
    qgiscraft_store::open_at(db.as_deref()).map_err(|e| e.to_string())
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "app": "qgiscraft-web",
        "version": env!("CARGO_PKG_VERSION"),
        "db": qgiscraft_store::default_db_path().to_string_lossy(),
    }))
}

async fn layers(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    let db = db_path(q.get("db").cloned());
    let conn = match open_db(&db) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    };
    let layers = match qgiscraft_store::list_layers(&conn) {
        Ok(l) => l,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    };
    let counts = qgiscraft_store::feature_counts(&conn).unwrap_or_default();
    let rows: Vec<Value> = layers
        .into_iter()
        .map(|l| {
            let n = counts
                .iter()
                .find(|(id, _)| *id == l.id)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            json!({"id": l.id, "name": l.name, "source": l.source, "geom_type": l.geom_type, "visible": l.visible, "features": n})
        })
        .collect();
    (
        StatusCode::OK,
        Json(json!({"count": rows.len(), "layers": rows})),
    )
}

#[derive(Debug, Deserialize, Default)]
struct FeaturesQuery {
    layer_id: Option<String>,
    db: Option<String>,
}

async fn features(Query(q): Query<FeaturesQuery>) -> impl IntoResponse {
    let conn = match open_db(&q.db) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    };
    match qgiscraft_store::list_features(&conn, q.layer_id.as_deref()) {
        Ok(rows) => (
            StatusCode::OK,
            Json(json!({"count": rows.len(), "features": rows})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

#[derive(Debug, Deserialize, Default)]
struct GeocodeQuery {
    query: Option<String>,
    limit: Option<u8>,
    live: Option<bool>,
}

async fn geocode(Query(q): Query<GeocodeQuery>) -> impl IntoResponse {
    let query = q.query.unwrap_or_default();
    if query.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid input: query required", "op": "geocode.search"})),
        );
    }
    let v = qgiscraft_core::dispatch(qgiscraft_core::Command::GeocodeSearch {
        query,
        limit: q.limit,
        live: q.live.unwrap_or(false),
    })
    .await;
    if v.get("error").is_some() {
        return (StatusCode::BAD_GATEWAY, Json(v));
    }
    (StatusCode::OK, Json(v))
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct OverpassQuery {
    key: Option<String>,
    value: Option<String>,
    south: Option<f64>,
    west: Option<f64>,
    north: Option<f64>,
    east: Option<f64>,
    live: Option<bool>,
    ingest: Option<bool>,
    db: Option<String>,
}

async fn overpass(Query(q): Query<OverpassQuery>) -> impl IntoResponse {
    let key = q.key.clone().unwrap_or_default();
    let value = q.value.clone().unwrap_or_default();
    let live = q.live.unwrap_or(false);
    let ingest = q.ingest.unwrap_or(false);
    if ingest && !live {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": "invalid input: ingest requires live=true", "op": "overpass.query"}),
            ),
        );
    }
    let out = qgiscraft_core::dispatch(qgiscraft_core::Command::OverpassQuery {
        key: key.clone(),
        value: value.clone(),
        south: q.south,
        west: q.west,
        north: q.north,
        east: q.east,
        live,
    })
    .await;
    if out.get("error").is_some() {
        return (StatusCode::BAD_GATEWAY, Json(out));
    }
    if !(ingest && live) {
        return (StatusCode::OK, Json(out));
    }
    let slug: String = format!("{key}-{value}")
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    let name = format!("{key}={value}");
    let hits = out.get("hits").cloned().unwrap_or(Value::Null);
    let stored = ingest_hits(q.db.as_deref(), &slug, &name, &hits);
    (
        StatusCode::OK,
        Json(json!({"query_result": out, "ingest": stored})),
    )
}

/// Shared ingest (mirrors the MCP layer): hits -> layer + features.
fn ingest_hits(db: Option<&str>, layer_id: &str, layer_name: &str, hits: &Value) -> Value {
    let conn = match qgiscraft_store::open_at(db) {
        Ok(c) => c,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let items = match hits.as_array() {
        Some(a) => a,
        None => return json!({"error": "invalid input: hits must be an array"}),
    };
    let mut feats = Vec::new();
    for h in items {
        let (Some(lng), Some(lat)) = (
            h.get("lng").and_then(|v| v.as_f64()),
            h.get("lat").and_then(|v| v.as_f64()),
        ) else {
            continue;
        };
        if !(-180.0..=180.0).contains(&lng) || !(-90.0..=90.0).contains(&lat) {
            continue;
        }
        feats.push(qgiscraft_store::StoredFeature {
            layer_id: layer_id.to_string(),
            name: h
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("(unnamed)")
                .to_string(),
            kind: h
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("node")
                .to_string(),
            lng,
            lat,
            geom_json: h
                .get("geom")
                .filter(|g| !g.is_null())
                .map(|g| g.to_string()),
        });
    }
    let layer = qgiscraft_store::StoredLayer::new(
        layer_id.to_string(),
        layer_name.to_string(),
        Some("http".to_string()),
        None,
        None,
    );
    if let Err(e) = qgiscraft_store::add_layer(&conn, &layer) {
        return json!({"error": e.to_string()});
    }
    match qgiscraft_store::replace_features(&conn, layer_id, &feats) {
        Ok(n) => json!({"layer": layer_id, "ingested": n}),
        Err(e) => json!({"error": e.to_string()}),
    }
}

#[derive(Debug, Deserialize, Default)]
struct PreviewQuery {
    width: Option<u32>,
    height: Option<u32>,
    tiles: Option<bool>,
    db: Option<String>,
}

/// Server-rendered map PNG (same projection math, headless BMP pipeline).
/// Lets a browser tab show exactly what the native canvas would draw.
async fn preview_png(Query(q): Query<PreviewQuery>) -> axum::response::Response {
    let err = |msg: String| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": msg})),
        )
            .into_response()
    };
    let w = q.width.unwrap_or(1280).clamp(64, 2048);
    let h = q.height.unwrap_or(860).clamp(64, 2048);
    let with_tiles = q.tiles.unwrap_or(true);
    let conn = match open_db(&q.db) {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    let template = {
        let layers = qgiscraft_store::list_layers(&conn).unwrap_or_default();
        layers
            .iter()
            .find(|l| l.visible && l.geom_type == "raster" && l.source.starts_with("xyz:"))
            .and_then(|l| l.source.strip_prefix("xyz:"))
            .map(|t| t.to_string())
            .unwrap_or_else(qgiscraft_core::tiles::tile_url_template)
    };
    // Fit visible features from this DB (same math as the canvas).
    let feats = qgiscraft_store::list_features(&conn, None).unwrap_or_default();
    // render_bmp does blocking tile I/O: run it on the blocking pool, not
    // the async worker (reqwest::blocking builds its own runtime and panics
    // if created - or dropped - inside an async context).
    let bytes =
        match tokio::task::spawn_blocking(move || render_bmp(&feats, w, h, with_tiles, &template))
            .await
        {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => return err(e),
            Err(e) => return err(format!("render task failed: {e}")),
        };
    // BMP (BGR, bottom-up) -> PNG via the image crate.
    // `bytes` is the finished BMP from the blocking render above.
    let stride = (w as usize * 3).div_ceil(4) * 4;
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    if bytes.len() < 54 + stride * h as usize {
        return err("bad bmp".to_string());
    }
    for y in 0..h as usize {
        for x in 0..w as usize {
            let src = 54 + y * stride + x * 3;
            let dst = ((h as usize - 1 - y) * w as usize + x) * 4;
            if src + 2 < bytes.len() && dst + 3 < rgba.len() {
                rgba[dst] = bytes[src + 2];
                rgba[dst + 1] = bytes[src + 1];
                rgba[dst + 2] = bytes[src];
                rgba[dst + 3] = 255;
            }
        }
    }
    let img = match image::RgbaImage::from_raw(w, h, rgba) {
        Some(img) => img,
        None => return err("rasterize failed".to_string()),
    };
    let mut png = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png);
    if image::DynamicImage::ImageRgba8(img)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .is_err()
    {
        return err("png encode failed".to_string());
    }
    match axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .body(axum::body::Body::from(png))
    {
        Ok(resp) => resp,
        Err(_) => err("response build failed".to_string()),
    }
}

/// Headless BMP via the same canvas math (duplicate of the app preview;
/// unified under one render module in a later pass).
fn render_bmp(
    feats: &[qgiscraft_store::StoredFeature],
    width: u32,
    height: u32,
    with_tiles: bool,
    template: &str,
) -> Result<Vec<u8>, String> {
    use qgiscraft_core::tiles;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("preview size must be 1..4096".to_string());
    }
    let (mut min_lng, mut max_lng, mut min_lat, mut max_lat) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for f in feats {
        if !f.lng.is_finite() || !f.lat.is_finite() {
            continue;
        }
        min_lng = min_lng.min(f.lng);
        max_lng = max_lng.max(f.lng);
        min_lat = min_lat.min(f.lat);
        max_lat = max_lat.max(f.lat);
    }
    if min_lng.is_infinite() {
        return Err("no features to frame".to_string());
    }
    let (clng, clat, zoom) = match tiles::fit_view(
        min_lng,
        min_lat,
        max_lng,
        max_lat,
        width as f32,
        height as f32,
    ) {
        Some(v) => v,
        None => return Err("cannot frame features".to_string()),
    };
    // Tiles: integer zoom nearest the float view zoom.
    let tile_z = (zoom.floor() as i64).clamp(0, 19) as u8;
    let (wlng, nlat) = unproject(width as f32, height as f32, clng, clat, zoom, 0.0, 0.0);
    let (elng, slat) = unproject(
        width as f32,
        height as f32,
        clng,
        clat,
        zoom,
        width as f32,
        height as f32,
    );
    let mut px = vec![8u8; (width as usize) * (height as usize) * 3];
    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 3) as usize;
            px[i] = 7;
            px[i + 1] = 5;
            px[i + 2] = 5;
        }
    }
    let mut plot = |x: i32, y: i32, b: u8, g: u8, r: u8| {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let i = (((height as i32 - 1 - y) as u32 * width + x as u32) * 3) as usize;
        if i + 2 < px.len() {
            px[i] = b;
            px[i + 1] = g;
            px[i + 2] = r;
        }
    };
    if with_tiles {
        if let Ok(list) = tiles::bbox_to_tiles(wlng, slat, elng, nlat, tile_z) {
            for t in list {
                let png = match tiles::fetch_tile_png_as(t, template) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let (tw, th, rgba) = match tiles::decode_png_rgba(&png) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let (tmin_lng, tmin_lat, tmax_lng, tmax_lat) = tiles::tile_bounds(t);
                let ax = project(
                    tmin_lng,
                    tmax_lat,
                    clng,
                    clat,
                    zoom,
                    width as f32,
                    height as f32,
                )
                .0;
                let ay = project(
                    tmin_lng,
                    tmax_lat,
                    clng,
                    clat,
                    zoom,
                    width as f32,
                    height as f32,
                )
                .1;
                let bx = project(
                    tmax_lng,
                    tmin_lat,
                    clng,
                    clat,
                    zoom,
                    width as f32,
                    height as f32,
                )
                .0;
                let by = project(
                    tmax_lng,
                    tmin_lat,
                    clng,
                    clat,
                    zoom,
                    width as f32,
                    height as f32,
                )
                .1;
                let (x0, y0) = (ax.floor() as i32, ay.floor() as i32);
                let (x1, y1) = (bx.ceil() as i32, by.ceil() as i32);
                let denom_x = (bx - ax).max(1.0);
                let denom_y = (by - ay).max(1.0);
                for dy in y0..=y1 {
                    for dx in x0..=x1 {
                        let sxx = (((dx as f32 - ax) / denom_x * tw as f32) as i32)
                            .clamp(0, tw as i32 - 1) as u32;
                        let syy = (((dy as f32 - ay) / denom_y * th as f32) as i32)
                            .clamp(0, th as i32 - 1) as u32;
                        let si = ((syy * tw + sxx) * 4) as usize;
                        if si + 2 < rgba.len() {
                            plot(dx, dy, rgba[si + 2], rgba[si + 1], rgba[si]);
                        }
                    }
                }
            }
        }
    }
    // Points + lines + polygons (same palette rule as the canvas).
    for f in feats {
        let color = layer_color(&f.layer_id);
        if let Some(g) = f.geom_json.as_ref() {
            if g.contains("\"LineString\"") || g.contains("\"Polygon\"") {
                continue; // full geometry renders in the canvas; preview draws points + tiles
            }
        }
        let (pxf, pyf) = project(f.lng, f.lat, clng, clat, zoom, width as f32, height as f32);
        let (cx, cy) = (pxf.round() as i32, pyf.round() as i32);
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy <= 6 {
                    plot(cx + dx, cy + dy, color.2, color.1, color.0);
                }
            }
        }
    }
    // BMP file: 14-byte header + 40-byte DIB + BGR rows (padded to 4).
    let row_stride = (width * 3).div_ceil(4) * 4;
    let img_size = row_stride * height;
    let file_size = 14 + 40 + img_size;
    let mut out = Vec::with_capacity(file_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 54, 0, 0, 0]);
    out.extend_from_slice(&[40, 0, 0, 0]);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&[1, 0, 24, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&img_size.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let pad = (row_stride - width * 3) as usize;
    for row in 0..height {
        let start = (row * width * 3) as usize;
        out.extend_from_slice(&px[start..start + (width * 3) as usize]);
        out.extend_from_slice(&vec![0u8; pad]);
    }
    Ok(out)
}

/// Thin adapters over core math: same signatures as before, one implementation.
fn unproject(w: f32, h: f32, clng: f64, clat: f64, zoom: f32, x: f32, y: f32) -> (f64, f64) {
    qgiscraft_core::tiles::unproject(x, y, clng, clat, zoom, w / 2.0, h / 2.0)
}

fn project(lng: f64, lat: f64, clng: f64, clat: f64, zoom: f32, w: f32, h: f32) -> (f32, f32) {
    qgiscraft_core::tiles::project(lng, lat, clng, clat, zoom, w / 2.0, h / 2.0)
}

fn layer_color(id: &str) -> (u8, u8, u8) {
    qgiscraft_core::tiles::layer_color(id)
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("QGISCRAFT_WEB_PORT")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(11254);
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/layers", get(layers))
        .route("/api/features", get(features))
        .route("/api/geocode", get(geocode))
        .route("/api/overpass", get(overpass))
        .route("/api/preview.png", get(preview_png))
        .fallback_service(ServeDir::new("web/dist"))
        .layer(CorsLayer::permissive());
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("bind {addr} failed: {e}");
            std::process::exit(1);
        });
    println!("qgiscraft-web on http://{addr} (Track B seed; dev console via :11253)");
    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("serve failed: {e}");
        std::process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_features() -> Vec<qgiscraft_store::StoredFeature> {
        vec![
            qgiscraft_store::StoredFeature {
                layer_id: "t".to_string(),
                name: "A".to_string(),
                kind: "node".to_string(),
                lng: 16.3,
                lat: 48.2,
                geom_json: None,
            },
            qgiscraft_store::StoredFeature {
                layer_id: "t".to_string(),
                name: "B".to_string(),
                kind: "node".to_string(),
                lng: 16.4,
                lat: 48.25,
                geom_json: None,
            },
        ]
    }

    #[test]
    fn preview_renders_offline_points() {
        let bmp = render_bmp(
            &demo_features(),
            320,
            200,
            false,
            "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        );
        assert!(bmp.is_ok());
        if let Ok(bytes) = bmp {
            assert!(bytes.len() > 54);
            assert_eq!(&bytes[0..2], b"BM");
        }
    }

    #[test]
    fn preview_rejects_empty() {
        assert!(render_bmp(&[], 320, 200, false, "https://x/{z}/{x}/{y}.png").is_err());
        assert!(render_bmp(&demo_features(), 0, 200, false, "https://x/{z}/{x}/{y}.png").is_err());
    }
}
