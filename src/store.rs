use bincode;
use bincode::rustc_serialize::{decode_from, encode_into};
use std::fs;
use std::path::{Path,PathBuf};

use super::{ Model, Effect };

use ::file_lock::Lock;

const CURRENT_VERSION: u32 = 1;
const PID_FILE: &'static str = "tasks.pid";

pub struct TaskStore {
  pub model: Model,

  effects_path: PathBuf,
  _file_lock: Lock,
}

impl TaskStore {
  pub fn new() -> Self {
    Self::load_from("effects.bin")
  }

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

    TaskStore {
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

impl Drop for TaskStore {
  fn drop(&mut self) {
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&self.effects_path)
      .expect("Failed to open tasks-file for writing");

    info!("Dropping TaskStore");
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

#[test]
fn test_serialization() {
  use std::{env, fs};
  use std::io::ErrorKind;
  use super::Task;

  let mut tempfile = env::temp_dir();
  tempfile.push("tasks.bin");
  match fs::remove_file(&tempfile) {
    Err(ref e) if e.kind() == ErrorKind::NotFound => (),
    Err(_) => panic!("Couldn't remove stale file `{:?}`", tempfile),
    _ => (),
  }

  let task = Task::new("task #1");
  {
    let mut store = TaskStore::load_from(&tempfile);
    assert_eq!(0, store.model.tasks.len());
    store.model.apply_effect(Effect::AddTask(task.clone()));
    assert_eq!(1, store.model.tasks.len());
    // store drops, gets serialized
  }
  {
    // Load from file, check if everything is as we've left it
    // TODO: Check for whole-model equality
    let store = TaskStore::load_from(&tempfile);
    assert_eq!(1, store.model.tasks.len());
    assert_eq!(Some(&task), store.model.tasks.get(&task.uuid));
  }

  fs::remove_file(tempfile).unwrap();
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
