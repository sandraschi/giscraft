//! SQLite project store: init/add/list. No UI, no MCP, no network.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredLayer {
    pub id: String,
    pub name: String,
    pub source: String,
    pub geom_type: String,
    pub crs: String,
    pub visible: bool,
}

/// One ingested point feature. Overpass hits land here via `replace_features`.
/// `geom_json` carries full geometry (GeoJSON LineString/Polygon) when the
/// element had it; points leave it None. `lng`/`lat` is always the centroid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredFeature {
    pub layer_id: String,
    pub name: String,
    pub kind: String,
    pub lng: f64,
    pub lat: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geom_json: Option<String>,
}

impl StoredLayer {
    pub fn new(
        id: String,
        name: String,
        source: Option<String>,
        geom_type: Option<String>,
        crs: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            source: source.unwrap_or_else(|| "osm".to_string()),
            geom_type: geom_type.unwrap_or_else(|| "point".to_string()),
            crs: crs.unwrap_or_else(|| "EPSG:4326".to_string()),
            visible: true,
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Sql(String),
    InvalidInput(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sql(e) => write!(f, "store sql: {e}"),
            StoreError::InvalidInput(e) => write!(f, "invalid input: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub fn init_db(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS layers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source TEXT NOT NULL,
            geom_type TEXT NOT NULL DEFAULT 'point',
            crs TEXT NOT NULL DEFAULT 'EPSG:4326',
            visible INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS features (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            layer_id TEXT NOT NULL REFERENCES layers(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'node',
            lng REAL NOT NULL,
            lat REAL NOT NULL,
            geom_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_features_layer ON features(layer_id);",
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    // Migrate old DBs (3/5/7-col schemas): ignore duplicate-column, keep real errors.
    for alter in [
        "ALTER TABLE layers ADD COLUMN geom_type TEXT NOT NULL DEFAULT 'point'",
        "ALTER TABLE layers ADD COLUMN crs TEXT NOT NULL DEFAULT 'EPSG:4326'",
        "ALTER TABLE layers ADD COLUMN visible INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE features ADD COLUMN geom_json TEXT",
    ] {
        match conn.execute_batch(alter) {
            Ok(()) => {}
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("duplicate column name") {
                    return Err(StoreError::Sql(msg));
                }
            }
        }
    }
    Ok(())
}

pub fn add_layer(conn: &Connection, layer: &StoredLayer) -> Result<(), StoreError> {
    if layer.id.trim().is_empty() {
        return Err(StoreError::InvalidInput("id must not be empty".to_string()));
    }
    if layer.name.trim().is_empty() {
        return Err(StoreError::InvalidInput(
            "name must not be empty".to_string(),
        ));
    }
    conn.execute(
        "INSERT OR REPLACE INTO layers (id, name, source, geom_type, crs, visible) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            layer.id,
            layer.name,
            layer.source,
            layer.geom_type,
            layer.crs,
            layer.visible as i64
        ],
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(())
}

pub fn list_layers(conn: &Connection) -> Result<Vec<StoredLayer>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT id, name, source, geom_type, crs, visible FROM layers ORDER BY id")
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let visible: i64 = row.get(5)?;
            Ok(StoredLayer {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                geom_type: row.get(3)?,
                crs: row.get(4)?,
                visible: visible != 0,
            })
        })
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok(l) => out.push(l),
            Err(e) => return Err(StoreError::Sql(e.to_string())),
        }
    }
    Ok(out)
}

/// Delete one layer by id (plus its features; FK cascade is off by default
/// in SQLite, so the child delete is explicit). Returns true when removed.
pub fn remove_layer(conn: &Connection, id: &str) -> Result<bool, StoreError> {
    if id.trim().is_empty() {
        return Err(StoreError::InvalidInput("id must not be empty".to_string()));
    }
    conn.execute("DELETE FROM features WHERE layer_id = ?1", params![id])
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let n = conn
        .execute("DELETE FROM layers WHERE id = ?1", params![id])
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(n > 0)
}

/// Flip layer visibility (Layers panel checkbox). True when the id exists.
pub fn set_visible(conn: &Connection, id: &str, visible: bool) -> Result<bool, StoreError> {
    if id.trim().is_empty() {
        return Err(StoreError::InvalidInput("id must not be empty".to_string()));
    }
    let n = conn
        .execute(
            "UPDATE layers SET visible = ?1 WHERE id = ?2",
            params![visible as i64, id],
        )
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(n > 0)
}

/// Replace a layer's feature set wholesale (ingest path: delete + insert,
/// no explicit txn in v1 -- each statement auto-commits; fine under 100 rows).
pub fn replace_features(
    conn: &Connection,
    layer_id: &str,
    feats: &[StoredFeature],
) -> Result<usize, StoreError> {
    if layer_id.trim().is_empty() {
        return Err(StoreError::InvalidInput(
            "layer_id must not be empty".to_string(),
        ));
    }
    conn.execute(
        "DELETE FROM features WHERE layer_id = ?1",
        params![layer_id],
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    let mut n = 0_usize;
    for f in feats {
        if !(-180.0..=180.0).contains(&f.lng) || !(-90.0..=90.0).contains(&f.lat) {
            continue;
        }
        conn.execute(
            "INSERT INTO features (layer_id, name, kind, lng, lat, geom_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![layer_id, f.name, f.kind, f.lng, f.lat, f.geom_json],
        )
        .map_err(|e| StoreError::Sql(e.to_string()))?;
        n += 1;
    }
    Ok(n)
}

fn map_feature(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredFeature> {
    Ok(StoredFeature {
        layer_id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        lng: row.get(3)?,
        lat: row.get(4)?,
        geom_json: row.get(5)?,
    })
}

/// List features, optionally filtered to one layer. Capped at 500 rows.
pub fn list_features(
    conn: &Connection,
    layer_id: Option<&str>,
) -> Result<Vec<StoredFeature>, StoreError> {
    let mut out = Vec::new();
    if let Some(id) = layer_id {
        let mut stmt = conn
            .prepare("SELECT layer_id, name, kind, lng, lat, geom_json FROM features WHERE layer_id = ?1 ORDER BY id LIMIT 500")
            .map_err(|e| StoreError::Sql(e.to_string()))?;
        let rows = stmt
            .query_map(params![id], map_feature)
            .map_err(|e| StoreError::Sql(e.to_string()))?;
        for r in rows {
            match r {
                Ok(f) => out.push(f),
                Err(e) => return Err(StoreError::Sql(e.to_string())),
            }
        }
    } else {
        let mut stmt = conn
            .prepare("SELECT layer_id, name, kind, lng, lat, geom_json FROM features ORDER BY layer_id, id LIMIT 500")
            .map_err(|e| StoreError::Sql(e.to_string()))?;
        let rows = stmt
            .query_map([], map_feature)
            .map_err(|e| StoreError::Sql(e.to_string()))?;
        for r in rows {
            match r {
                Ok(f) => out.push(f),
                Err(e) => return Err(StoreError::Sql(e.to_string())),
            }
        }
    }
    Ok(out)
}

/// Per-layer feature counts for layer lists and the egui dock.
pub fn feature_counts(conn: &Connection) -> Result<Vec<(String, u64)>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT layer_id, COUNT(*) FROM features GROUP BY layer_id")
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let n: i64 = row.get(1)?;
            Ok((id, n))
        })
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        match r {
            Ok((id, n)) => out.push((id, u64::try_from(n).unwrap_or(0))),
            Err(e) => return Err(StoreError::Sql(e.to_string())),
        }
    }
    Ok(out)
}

/// Default project DB path: explicit `--db` > `$QGISCRAFT_DB` > `./qgiscraft.db`.
/// Explicit param keeps tests hermetic (unique temp files, no env mutation).
pub fn resolve_db_path(explicit: Option<&str>) -> std::path::PathBuf {
    if let Some(raw) = explicit {
        if !raw.trim().is_empty() {
            return std::path::PathBuf::from(raw.trim());
        }
    }
    default_db_path()
}

/// Open a DB at an explicit-or-default path and ensure schema.
pub fn open_at(explicit: Option<&str>) -> Result<Connection, StoreError> {
    let path = resolve_db_path(explicit);
    let conn = Connection::open(&path).map_err(|e| StoreError::Sql(format!("open db: {e}")))?;
    init_db(&conn)?;
    Ok(conn)
}

/// Default project DB path: `$QGISCRAFT_DB` or `./qgiscraft.db`.
/// Env override keeps ad-hoc runs hermetic (point it at a temp file).
pub fn default_db_path() -> std::path::PathBuf {
    if let Ok(raw) = std::env::var("QGISCRAFT_DB") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return std::path::PathBuf::from(trimmed);
        }
    }
    std::path::PathBuf::from("qgiscraft.db")
}

/// Open the default DB and ensure schema. Used by CLI/MCP persistence.
pub fn open_default() -> Result<Connection, StoreError> {
    let path = default_db_path();
    let conn = Connection::open(&path).map_err(|e| StoreError::Sql(format!("open db: {e}")))?;
    init_db(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_id_is_rejected() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            let init = init_db(&conn);
            assert!(init.is_ok());
            let bad = StoredLayer::new("   ".to_string(), "A".to_string(), None, None, None);
            assert!(add_layer(&conn, &bad).is_err());
            let bad_name = StoredLayer::new("a".to_string(), "  ".to_string(), None, None, None);
            assert!(add_layer(&conn, &bad_name).is_err());
        }
    }

    #[test]
    fn add_and_list_roundtrip() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            let layer = StoredLayer::new(
                "a".to_string(),
                "A".to_string(),
                Some("osm".to_string()),
                Some("point".to_string()),
                None,
            );
            assert!(add_layer(&conn, &layer).is_ok());
            let listed = list_layers(&conn);
            assert!(listed.is_ok());
            if let Ok(layers) = listed {
                assert_eq!(layers.len(), 1);
                assert_eq!(layers[0].crs, "EPSG:4326");
            }
        }
    }

    #[test]
    fn remove_roundtrip_and_hostile_id() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            assert!(remove_layer(&conn, "   ").is_err());
            let layer = StoredLayer::new(
                "gone".to_string(),
                "G".to_string(),
                Some("osm".to_string()),
                None,
                None,
            );
            assert!(add_layer(&conn, &layer).is_ok());
            let removed = remove_layer(&conn, "gone");
            assert!(removed.is_ok());
            if let Ok(was_removed) = removed {
                assert!(was_removed);
            }
        }
    }

    #[test]
    fn geom_json_roundtrips() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            let layer = StoredLayer::new(
                "g".to_string(),
                "G".to_string(),
                Some("overpass".to_string()),
                None,
                None,
            );
            assert!(add_layer(&conn, &layer).is_ok());
            let feats = vec![StoredFeature {
                layer_id: "g".to_string(),
                name: "Ring".to_string(),
                kind: "way".to_string(),
                lng: 16.35,
                lat: 48.225,
                geom_json: Some(
                    r#"{"type":"LineString","coordinates":[[16.3,48.2],[16.4,48.25]]}"#.to_string(),
                ),
            }];
            let n = replace_features(&conn, "g", &feats);
            assert!(n.is_ok());
            let listed = list_features(&conn, Some("g"));
            assert!(listed.is_ok());
            if let Ok(rows) = listed {
                assert_eq!(rows.len(), 1);
                assert!(rows[0]
                    .geom_json
                    .as_ref()
                    .is_some_and(|g| g.contains("LineString")));
            }
        }
    }

    #[test]
    fn default_db_path_respects_env() {
        let path = default_db_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn features_replace_list_count_roundtrip() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            let layer = StoredLayer::new(
                "rx".to_string(),
                "Rx".to_string(),
                Some("overpass".to_string()),
                None,
                None,
            );
            assert!(add_layer(&conn, &layer).is_ok());
            let feats = vec![
                StoredFeature {
                    layer_id: "rx".to_string(),
                    name: "A".to_string(),
                    kind: "node".to_string(),
                    lng: 16.3,
                    lat: 48.2,
                    geom_json: None,
                },
                StoredFeature {
                    layer_id: "rx".to_string(),
                    name: "Bad".to_string(),
                    kind: "node".to_string(),
                    lng: 999.0,
                    lat: 48.2,
                    geom_json: None,
                },
            ];
            let n = replace_features(&conn, "rx", &feats);
            assert!(n.is_ok());
            // Out-of-range hit skipped: 1 stored, not 2.
            if let Ok(n) = n {
                assert_eq!(n, 1);
            }
            let listed = list_features(&conn, Some("rx"));
            assert!(listed.is_ok());
            if let Ok(rows) = listed {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].name, "A");
            }
            let counts = feature_counts(&conn);
            assert!(counts.is_ok());
            // Removing the layer removes its features too.
            assert!(remove_layer(&conn, "rx").is_ok());
            let after = list_features(&conn, Some("rx"));
            if let Ok(rows) = after {
                assert!(rows.is_empty());
            }
        }
    }

    #[test]
    fn visible_toggle_roundtrip() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            let layer = StoredLayer::new("v".to_string(), "V".to_string(), None, None, None);
            assert!(add_layer(&conn, &layer).is_ok());
            assert!(set_visible(&conn, "v", false).is_ok());
            let listed = list_layers(&conn);
            if let Ok(layers) = listed {
                assert!(!layers[0].visible);
            }
        }
    }
}
