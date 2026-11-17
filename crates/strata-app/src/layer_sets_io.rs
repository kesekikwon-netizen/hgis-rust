//! Explicit-save metadata boundary, target/cpp-ref/src/app/KaDrawingLayerSets.cpp:29-59.
use super::{Catalog, LayerSetState};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;

const TABLE: &str = "ka_hgis_layer_sets";
const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    sets: Catalog,
}

fn connection(file: &Path, flags: OpenFlags) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(file, flags).map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::ZERO).map_err(|e| e.to_string())?;
    Ok(conn)
}

fn read_catalog(conn: &Connection) -> Result<Option<Catalog>, String> {
    let exists: bool =
        conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 COLLATE NOCASE)", [TABLE], |r| r.get(0)).map_err(|e| e.to_string())?;
    if !exists {
        return Ok(None);
    }
    let count: i64 = conn.query_row(&format!("SELECT count(*) FROM {TABLE}"), [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(Some(Catalog::new()));
    }
    if count != 1 {
        return Err("invalid layer-set metadata row count".to_owned());
    }
    let payload: String = conn.query_row(&format!("SELECT payload FROM {TABLE} WHERE id=1"), [], |r| r.get(0)).map_err(|e| e.to_string())?;
    let document: Document = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
    if document.version != VERSION {
        return Err(format!("unsupported layer-set metadata version {}", document.version));
    }
    if document.sets.iter().any(|(name, checks)| name.is_empty() || name.trim() != name || checks.keys().any(String::is_empty)) {
        return Err("invalid layer-set metadata name or layer key".to_owned());
    }
    Ok(Some(document.sets))
}

pub(crate) fn read(file: &Path) -> Result<LayerSetState, String> {
    let conn = connection(file, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(LayerSetState { catalog: read_catalog(&conn)?.unwrap_or_default(), ..Default::default() })
}

pub(crate) fn write(state: &LayerSetState, file: &Path) -> Result<(), String> {
    let mut conn = connection(file, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let existing = read_catalog(&tx)?;
    if existing.is_none() && state.catalog.is_empty() {
        return Ok(());
    }
    let payload = serde_json::to_string(&Document { version: VERSION, sets: state.catalog.clone() }).map_err(|e| e.to_string())?;
    // target/cpp-ref/src/app/KaDrawingLayerSets.cpp:39-40; use the existing Rust GPKG metadata-table pattern (gpkg_repository.rs:120-125).
    tx.execute_batch(&format!("CREATE TABLE IF NOT EXISTS {TABLE} (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL)")).map_err(|e| e.to_string())?;
    tx.execute(&format!("INSERT INTO {TABLE} (id,payload) VALUES (1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload"), [payload]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
