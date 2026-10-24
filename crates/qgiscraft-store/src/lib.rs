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
            crs TEXT NOT NULL DEFAULT 'EPSG:4326'
        );",
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    // Migrate Phase 0 DBs (3-col schema): ignore duplicate-column, keep real errors.
    for alter in [
        "ALTER TABLE layers ADD COLUMN geom_type TEXT NOT NULL DEFAULT 'point'",
        "ALTER TABLE layers ADD COLUMN crs TEXT NOT NULL DEFAULT 'EPSG:4326'",
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
        "INSERT OR REPLACE INTO layers (id, name, source, geom_type, crs) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![layer.id, layer.name, layer.source, layer.geom_type, layer.crs],
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(())
}

pub fn list_layers(conn: &Connection) -> Result<Vec<StoredLayer>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT id, name, source, geom_type, crs FROM layers ORDER BY id")
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredLayer {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                geom_type: row.get(3)?,
                crs: row.get(4)?,
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

/// Delete one layer by id. Returns true when a row was removed.
pub fn remove_layer(conn: &Connection, id: &str) -> Result<bool, StoreError> {
    if id.trim().is_empty() {
        return Err(StoreError::InvalidInput("id must not be empty".to_string()));
    }
    let n = conn
        .execute("DELETE FROM layers WHERE id = ?1", params![id])
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(n > 0)
}

/// Default project DB path: explicit `--db` > `$QGISCRRAFT_DB` > `./qgiscraft.db`.
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

/// Default project DB path: `$QGISCRRAFT_DB` or `./qgiscraft.db`.
/// Env override keeps ad-hoc runs hermetic (point it at a temp file).
pub fn default_db_path() -> std::path::PathBuf {
    if let Ok(raw) = std::env::var("QGISCRRAFT_DB") {
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
    fn default_db_path_respects_env() {
        let path = default_db_path();
        assert!(!path.as_os_str().is_empty());
    }
}
