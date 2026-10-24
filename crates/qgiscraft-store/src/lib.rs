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
}
