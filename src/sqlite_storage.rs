use std::collections::BTreeMap;
use std::path::Path;

use serde_json;

use rusqlite;
use rusqlite::Connection;

use StorageEngine;
use {Effect, Model, Uuid};

pub struct SqliteStorage {
    model: Model,
    db: Connection,
}

#[derive(Debug, Fail, From)]
pub enum Error {
    #[fail(display = "Sqlite Error: {}", _0)]
    Sqlite(rusqlite::Error),
    #[fail(display = "Json Error: {}", _0)]
    Json(serde_json::Error)
}

impl SqliteStorage {
    fn is_initialized(db: &Connection) -> bool {
        db.query_row(
            "SELECT * FROM sqlite_master
                  WHERE name = 'effects'
                  AND   type = 'table'",
            &[],
            |_| 0,
        ).is_ok()
    }

    fn initialize_db(db: &mut Connection) -> Result<(), Error> {
        assert_eq!(false, Self::is_initialized(&db));

        info!("Initializing SQL Storage");

        // TODO: There's a new function rusqlite for this
        let schema = include_str!("schema.sql");
        let commands = schema
            .split("\n\n")
            .map(str::trim)
            .filter(|s| !s.is_empty());

        for command in commands {
            debug!("Executing SQL: {:?}", command);
            try!(db.execute(&format!("{};", command), &[]));
        }
        Ok(())
    }

    fn query_effects(db: &Connection) -> Result<Vec<Effect>, Error> {
        let mut stmt = try!(db.prepare("select * from effects order by id"));

        let rows = stmt.query_map(&[], |row| row.get(1))?;
        let effects: Vec<Effect> = rows.map(|json_str| {
            let json_str: String = json_str?;
            let json = serde_json::from_str(&json_str)?;

            Ok(json)
        }).collect::<Result<Vec<_>, Error>>()?;

        debug!("effects: #{:?}", effects);

        Ok(effects)
    }

    fn query_numerical_ids(db: &Connection) -> Result<Vec<(String, u64, Uuid)>, Error> {
        let mut stmt = try!(db.prepare("select scope, id, uuid from numerical_ids"));

        let rows = try!(stmt.query_map(&[], |row| {
            let scope: String = row.get(0);
            let n: i64 = row.get(1);
            let uuid: String = row.get(2);
            (scope, n as u64, uuid)
        }));
        
        let uuids = rows.map(|row| {
            let (scope, n, uuid_str) = row?;
            Ok((scope, n, serde_json::from_str(&uuid_str)?))
        }).collect::<Result<Vec<_>, Error>>()?;

        debug!("numerical_ids: {:?}", uuids);

        Ok(uuids)
    }

    fn load_from<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut db = try!(Connection::open(path));

        if !Self::is_initialized(&db) {
            try!(Self::initialize_db(&mut db));
        }

        let effects = try!(Self::query_effects(&db));
        let mut model = Model::from_effects(&effects);

        info!("Loaded {} tasks from disk", model.tasks.len());

        // Numerical ID Resolving
        let numerical_ids = try!(Self::query_numerical_ids(&db));

        for (scope, id, uuid) in numerical_ids {
            let mut inner = model.numerical_ids.entry(scope).or_insert(BTreeMap::new());
            inner.insert(id, uuid);
        }

        Ok(SqliteStorage {
            model: model,
            db: db,
        })
    }

    pub fn new_in_memory() -> Self {
        SqliteStorage {
            model: Model::new(),
            db: Connection::open_in_memory().unwrap(),
        }
    }
}

impl StorageEngine for SqliteStorage {
    type LoadErr = Error;

    fn new() -> Result<Self, Self::LoadErr> {
        Self::load_from("store.sqlite")
    }

    fn model<'a>(&'a mut self) -> &'a mut Model {
        &mut self.model
    }
}

impl Drop for SqliteStorage {
    fn drop(&mut self) {
        if !self.model.is_dirty() {
            info!("Not serializing as model isn't dirty");
            return;
        }

        // Ugh
        let tx = self.db.transaction().expect("Failed to create transacton");

        let row_count: i64 = tx.query_row("select count(id) from effects", &[], |row| row.get(0))
            .unwrap();
        debug!("Got {} rows", row_count);

        for (n, effect) in self.model.applied_effects.iter().enumerate() {
            if n >= row_count as usize {
                let json = serde_json::to_string(&effect).unwrap();
                debug!("Inserting JSON: {:?}", json);
                tx.execute("insert into effects (json) values ($1)", &[&json])
                    .unwrap();
            } else {
                debug!("Skipping row {}", n);
            }
        }

        debug!("Storing numerical_ids");
        tx.execute("delete from numerical_ids", &[])
            .expect("Failed to clear numerical_ids");

        for (scope, ids) in self.model.numerical_ids.iter() {
            for (n, uuid) in ids {
                let n = *n as i64;
                let uuid = serde_json::to_string(&uuid).unwrap();
                tx.execute(
                    "insert into numerical_ids (scope, id, uuid) values ($1, $2, $3)",
                    &[scope, &n, &uuid],
                ).expect("Failed to insert numerical id");
            }
        }

        tx.commit().expect("Failed to commit transaction");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use {Effect, Task};

    #[test]
    fn test_serialization() {
        use std::io::ErrorKind;
        use std::{env, fs, mem};

        let mut tempfile = env::temp_dir();
        tempfile.push("tasks.bin");
        match fs::remove_file(&tempfile) {
            Err(ref e) if e.kind() == ErrorKind::NotFound => (),
            Err(_) => panic!("Couldn't remove stale file `{:?}`", tempfile),
            _ => (),
        }

        let task = Task::new("task #1");
        let mut store = SqliteStorage::load_from(&tempfile).unwrap();
        assert_eq!(0, store.model.tasks.len());
        store.model.apply_effect(&Effect::AddTask(task.clone()));
        assert_eq!(1, store.model.tasks.len());
        mem::drop(store); // store drops, gets serialized

        // Load from file, check if everything is as we've left it
        // TODO: Check for whole-model equality
        let store = SqliteStorage::load_from(&tempfile).unwrap();
        assert_eq!(1, store.model.tasks.len());
        assert_eq!(Some(&task), store.model.tasks.get(&task.uuid));

        fs::remove_file(tempfile).unwrap();
    }
}
