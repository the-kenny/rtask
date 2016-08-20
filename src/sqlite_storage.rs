use std::path::Path;

use rusqlite::Connection;
use rustc_serialize::json;

use ::file_lock::Lock;
use ::{ Model, Effect };
use ::StorageEngine;
use ::storage_engine::PID_FILE;

pub struct SqliteStorage {
  model: Model,

  db: Connection,
  _file_lock: Option<Lock>,
}

impl SqliteStorage {
  fn is_initialized(db: &Connection) -> bool {
    db.query_row("SELECT * FROM sqlite_master
                  WHERE name = 'effects'
                  AND   type = 'table'",
                 &[], |_| 0)
      .is_ok()
  }

  fn initialize_db(db: &mut Connection) {
    assert!(!Self::is_initialized(&db));

    info!("Initializing SQL Storage");

    let schema = include_str!("schema.sql");
    let commands = schema.split(";").map(str::trim).filter(|s| !s.is_empty());
    for command in commands {
      debug!("Executing SQL: {:?}", command);
      db.execute(&format!("{};", command), &[]).unwrap();
    }
  }

  // TODO: Result
  fn query_effects(db: &Connection) -> Vec<Effect> {
    let mut stmt = db.prepare("select * from effects order by id").unwrap();

    let effects: Vec<Effect> = stmt.query_map(&[], |row| row.get(1)).unwrap().map(|row| {
      let data: String = row.unwrap();
      json::decode(&data).unwrap()
    }).collect();

    debug!("effects: #{:?}", effects);

    effects
  }

  fn load_from<P: AsRef<Path>>(path: P) -> Self {
    let lock = Lock::new(Path::new(PID_FILE))
      .expect("Couldn't acquire lock");

    let mut db = Connection::open(path)
      .expect("Failed to open db");

    if !Self::is_initialized(&db) {
      Self::initialize_db(&mut db);
    }

    let effects = Self::query_effects(&db);
    let model = Model::from_effects(effects);

    info!("Loaded {} tasks from disk", model.tasks.len());

    SqliteStorage {
      model: model,
      db: db,
      _file_lock: Some(lock)
    }
  }

  pub fn new_in_memory() -> Self {
    SqliteStorage {
      model: Model::new(),
      db: Connection::open_in_memory().unwrap(),
      _file_lock: None,
    }
  }

}

impl StorageEngine for SqliteStorage {
  type LoadErr = ();

  fn new() -> Result<Self, Self::LoadErr> {
    // TODO: error handling
    Ok(Self::load_from("store.sqlite"))
  }

  fn model<'a>(&'a mut self) -> &'a mut Model {
    &mut self.model
  }
}

impl Drop for SqliteStorage {
  fn drop(&mut self) {
    if !self.model.is_dirty() {
      info!("Not serializing as model isn't dirty");
      return
    }

    // Ugh
    let tx = self.db.transaction()
      .expect("Failed to create transacton");

    let row_count: i64 = tx.query_row("select count(id) from effects", &[], |row| row.get(0)).unwrap();
    debug!("Got {} rows", row_count);

    for (n, effect) in self.model.applied_effects.iter().enumerate() {
      if n >= row_count as usize {
        let json = json::encode(&effect).unwrap();
        debug!("Inserting JSON: {:?}", json);
        tx.execute("insert into effects (json) values ($1)", &[&json])
          .unwrap();
      } else {
        debug!("Skipping row {}", n);
      }
    }

    tx.commit().expect("Failed to commit transaction");
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::{Effect,Task};

  #[test]
  fn test_serialization() {
    use std::{env, fs};
    use std::io::ErrorKind;

    let mut tempfile = env::temp_dir();
    tempfile.push("tasks.bin");
    match fs::remove_file(&tempfile) {
      Err(ref e) if e.kind() == ErrorKind::NotFound => (),
      Err(_) => panic!("Couldn't remove stale file `{:?}`", tempfile),
      _ => (),
    }

    let task = Task::new("task #1");
    {
      let mut store = SqliteStorage::load_from(&tempfile);
      assert_eq!(0, store.model.tasks.len());
      store.model.apply_effect(Effect::AddTask(task.clone()));
      assert_eq!(1, store.model.tasks.len());
      // store drops, gets serialized
    }
    {
      // Load from file, check if everything is as we've left it
      // TODO: Check for whole-model equality
      let store = SqliteStorage::load_from(&tempfile);
      assert_eq!(1, store.model.tasks.len());
      assert_eq!(Some(&task), store.model.tasks.get(&task.uuid));
    }

    fs::remove_file(tempfile).unwrap();
  }
}
