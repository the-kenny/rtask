use bincode;
use bincode::rustc_serialize::{decode_from, encode_into};
use std::fs;
use std::path::{Path,PathBuf};

use rusqlite::Connection;

use rustc_serialize::json;

use super::{ Model, Effect };

use ::file_lock::Lock;

const CURRENT_VERSION: u32 = 1;
const PID_FILE: &'static str = "tasks.pid";

pub trait StoreEngine: Sized + Drop {
  type LoadErr;
  // fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, Self::LoadErr>;
  fn new() -> Result<Self, Self::LoadErr>;
  fn model<'a>(&'a mut self) -> &'a mut Model;
}

// Sqlite

pub struct SqliteStore {
  model: Model,

  db: Connection,
  _file_lock: Lock,
}

impl SqliteStore {
  fn is_initialized(db: &Connection) -> bool {
    db.query_row("SELECT * FROM sqlite_master 
                  WHERE name = 'effects' 
                  AND   type = 'table'",
                 &[], |_| 0)
      .is_ok()
  }
  
  fn initialize_db(db: &mut Connection) {
    assert!(!Self::is_initialized(&db));

    let schema = include_str!("schema.sql");
    for command in schema.split(";") {
      debug!("Executing SQL: {:?}", command);
      db.execute(&format!("{};", command), &[]).unwrap();
    }
  }

  // TODO: Result
  fn query_effects(db: &Connection) -> Vec<Effect> {
    let mut stmt = db.prepare("select * from effects order by id").unwrap();
    
    let effects: Vec<Effect> = stmt.query_map(&[], |row| (row.get(0), row.get(1))).unwrap().map(|row| {
      let row = row.unwrap();
      let id: i64 = row.0;
      let data: String = row.1;
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

    SqliteStore {
      model: model,
      db: db,
      _file_lock: lock
    }
  }
  
}

impl StoreEngine for SqliteStore {
  type LoadErr = ();

  fn new() -> Result<Self, Self::LoadErr> {
    // TODO: error handling
    Ok(Self::load_from("store.sqlite"))
  }
  
  fn model<'a>(&'a mut self) -> &'a mut Model {
    &mut self.model
  }
}

impl Drop for SqliteStore {
  fn drop(&mut self) {
    // Ugh
    let tx = self.db.transaction()
      .expect("Failed to create transacton");

    tx.execute("delete from effects", &[]).unwrap();
    for effect in &self.model.applied_effects {
      let json = json::encode(&effect).unwrap();
      debug!("Inserting JSON: {:?}", json);
      tx.execute("insert into effects (json) values ($1)", &[&json])
        .unwrap();
    }

    tx.commit();
  }
}

// rustc-serialize

pub struct TrivialStore {
  model: Model,
  effects_path: PathBuf,
  _file_lock: Lock,
}

impl StoreEngine for TrivialStore {
  type LoadErr = ();
  fn new() -> Result<Self, Self::LoadErr> {
    Ok(Self::load_from("effects.bin"))
  }
  
  fn model<'a>(&'a mut self) -> &'a mut Model {
    &mut self.model
  }
}

impl TrivialStore {
  fn load_from<P: AsRef<Path>>(path: P) -> Self {
    let lock = Lock::new(Path::new(PID_FILE))
      .expect("Couldn't acquire lock");

    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&path)
      .expect("Couldn't access tasks.bin");

    let meta: fs::Metadata = file.metadata().expect("Couldn't get file metadata");
    let model = if meta.len() > 0 {
      let disk_store = DiskStore::new_from(&mut file).unwrap();
      Model::from_effects(disk_store.effects)
    } else {
      Model::new()
    };

    info!("Loaded {} tasks from disk", model.tasks.len());

    TrivialStore {
      model: model,
      effects_path: path.as_ref().to_path_buf(),

      _file_lock: lock
    }
  }

  fn serialize(&self) -> DiskStore {
    DiskStore {
      effects: self.model.applied_effects.clone()
    }
  }
}

impl Drop for TrivialStore {
  fn drop(&mut self) {
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&self.effects_path)
      .expect("Failed to open tasks-file for writing");

    info!("Dropping TrivialStore");
    if self.model.is_dirty() {
      info!("Serializing effects to disk");
      self.serialize().write(&mut file).unwrap();
    } else {
      info!("Not serializing as nothing has changed")
    }

    fs::remove_file(PID_FILE)
      .expect("Failed to remove PID file");
  }
}

// On-Disk representation
#[derive(RustcEncodable, RustcDecodable)]
struct DiskStore {
  effects: Vec<Effect>,
}

use std::io::{Read,Write};
use bincode::rustc_serialize::{EncodingResult,DecodingResult};

impl DiskStore {
  fn write<W: Write>(&self, writer: &mut W) -> EncodingResult<()> {
    try!(encode_into(&CURRENT_VERSION, writer, bincode::SizeLimit::Infinite));
    try!(encode_into(self, writer, bincode::SizeLimit::Infinite));
    Ok(())
  }

  fn new_from<R: Read>(reader: &mut R) -> DecodingResult<Self> {
    let version: u32 = try!(decode_from(reader, bincode::SizeLimit::Bounded(4)));
    debug!("DiskStore.version {}", version);
    if version != CURRENT_VERSION {
      panic!("Incompatible on-disk version: {}", version);
    }

    let store: DiskStore = try!(decode_from(reader, bincode::SizeLimit::Infinite));
    debug!("Got {} effects in DiskStore", store.effects.len());
    for t in &store.effects { debug!("{:?}", t); }

    Ok(store)
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
      let mut store = TrivialStore::load_from(&tempfile);
      assert_eq!(0, store.model.tasks.len());
      store.model.apply_effect(Effect::AddTask(task.clone()));
      assert_eq!(1, store.model.tasks.len());
      // store drops, gets serialized
    }
    {
      // Load from file, check if everything is as we've left it
      // TODO: Check for whole-model equality
      let store = TrivialStore::load_from(&tempfile);
      assert_eq!(1, store.model.tasks.len());
      assert_eq!(Some(&task), store.model.tasks.get(&task.uuid));
    }

    fs::remove_file(tempfile).unwrap();
  }
}
