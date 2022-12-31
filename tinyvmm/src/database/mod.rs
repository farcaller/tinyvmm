use log::trace;
use regex::Regex;
use rusqlite::{
    functions::{Context, FunctionFlags},
    Connection, Error,
};
use serde_json::Value;
use std::path::PathBuf;

pub mod bridge;
pub mod entity;
pub mod error;
pub mod metadata;
pub mod virtual_machine;

const DATA_DIR: &str = "/var/lib/tinyvmm";
const DB_PATH: &str = "db.sqlite";

fn regexp_with_auxiliary(ctx: &Context<'_>) -> rusqlite::Result<bool> {
    assert_eq!(ctx.len(), 2, "called with unexpected number of arguments");
    type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
    let regexp: std::sync::Arc<Regex> = ctx.get_or_create_aux(0, |vr| -> Result<_, BoxError> {
        Ok(Regex::new(vr.as_str()?)?)
    })?;

    let is_match = {
        let text = ctx
            .get_raw(1)
            .as_str()
            .map_err(|e| Error::UserFunctionError(e.into()))?;

        regexp.is_match(text)
    };

    Ok(is_match)
}

fn get_connection() -> rusqlite::Result<Connection> {
    let conn = Connection::open(PathBuf::from(DATA_DIR).join(DB_PATH))?;

    conn.create_scalar_function(
        "regexp",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        regexp_with_auxiliary,
    )?;

    conn.execute_batch(
        r"
            BEGIN;
            CREATE TABLE IF NOT EXISTS entities (
            key TEXT GENERATED ALWAYS AS (kind || '/' || name) VIRTUAL UNIQUE,

            kind TEXT GENERATED ALWAYS AS (LOWER(json_extract(data, '$.kind'))) VIRTUAL NOT NULL,
            name TEXT GENERATED ALWAYS
                AS (json_extract(data, '$.metadata.name')) VIRTUAL
                NOT NULL,

            data  TEXT NOT NULL,

            CHECK (name REGEXP '[a-z0-9]([-a-z0-9]*[a-z0-9])?(\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*')
            CHECK (kind REGEXP '[A-Za-z]([A-Za-z0-9])*')
        );

        CREATE INDEX IF NOT EXISTS entities_kind ON entities (kind);
        CREATE INDEX IF NOT EXISTS entities_name ON entities (name);
        CREATE UNIQUE INDEX IF NOT EXISTS entities_key ON entities (key);
        END;",
    )?;

    Ok(conn)
}

pub fn get_entity(kind: &str, name: &str) -> rusqlite::Result<Value> {
    let conn = get_connection()?;

    let key = format!("{}/{}", kind.to_lowercase(), name.to_lowercase());

    // TODO: this returns a weird 'parse error: Invalid numeric literal at line 1, column 9' when key not found
    conn.query_row_and_then("SELECT data FROM entities WHERE key = ?1", [key], |row| {
        let val = row.get(0);
        trace!("queried {}/{}: {:?}", kind, name, val);
        val
    })
}

pub fn delete_entity(kind: &str, name: &str) -> rusqlite::Result<()> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare("DELETE FROM entities WHERE key = ?1")?;

    let key = format!("{}/{}", kind.to_lowercase(), name.to_lowercase());

    stmt.execute([key])?;

    Ok(())
}

pub fn get_kind(kind: &str) -> rusqlite::Result<Vec<Value>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare("SELECT data FROM entities WHERE kind = ?1")?;

    let result = stmt.query_map([kind.to_lowercase()], |row| {
        let val: Value = row.get(0)?;
        Ok(val)
    })?;

    let mut entities = vec![];
    for res in result {
        match res {
            Ok(e) => entities.push(e),
            Err(e) => return Err(e),
        }
    }
    Ok(entities)
}

pub fn create_entity(e: Value) -> rusqlite::Result<()> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare("INSERT INTO entities (data) VALUES (?1)")?;

    stmt.execute([e])?;

    Ok(())
}
