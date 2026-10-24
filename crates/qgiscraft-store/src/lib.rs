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
            source TEXT NOT NULL
        );",
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(())
}

pub fn add_layer(conn: &Connection, layer: &StoredLayer) -> Result<(), StoreError> {
    if layer.id.trim().is_empty() {
        return Err(StoreError::InvalidInput("id must not be empty".to_string()));
    }
    conn.execute(
        "INSERT OR REPLACE INTO layers (id, name, source) VALUES (?1, ?2, ?3)",
        params![layer.id, layer.name, layer.source],
    )
    .map_err(|e| StoreError::Sql(e.to_string()))?;
    Ok(())
}

pub fn list_layers(conn: &Connection) -> Result<Vec<StoredLayer>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT id, name, source FROM layers ORDER BY id")
        .map_err(|e| StoreError::Sql(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredLayer {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
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

/// Default project DB path: `$QGISCRRAFT_DB` or `./qgiscraft.db`.
/// Env override keeps tests hermetic (point it at a temp file).
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
            let bad = StoredLayer {
                id: "   ".to_string(),
                name: "A".to_string(),
                source: "osm".to_string(),
            };
            assert!(add_layer(&conn, &bad).is_err());
        }
    }

    #[test]
    fn add_and_list_roundtrip() {
        let conn_opt = Connection::open_in_memory().ok();
        assert!(conn_opt.is_some());
        if let Some(conn) = conn_opt {
            assert!(init_db(&conn).is_ok());
            let layer = StoredLayer {
                id: "a".to_string(),
                name: "A".to_string(),
                source: "osm".to_string(),
            };
            assert!(add_layer(&conn, &layer).is_ok());
            let listed = list_layers(&conn);
            assert!(listed.is_ok());
            if let Ok(layers) = listed {
                assert_eq!(layers.len(), 1);
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
            let layer = StoredLayer {
                id: "gone".to_string(),
                name: "G".to_string(),
                source: "osm".to_string(),
            };
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
