use bincode;
use bincode::rustc_serialize::{decode_from, encode_into};
use std::fs;
use std::path::{Path,PathBuf};
use std::io::{BufWriter};
use std::iter::FromIterator;

use ::task::{Task, Uuid};
use ::file_lock::Lock;

use std::collections::{HashMap};

pub struct TaskStore {
  pub tasks: HashMap<Uuid, Task>, // TODO: Make private
  tasks_path: PathBuf,
  file_lock: Lock,
}

impl TaskStore {
  pub fn load() -> Self {
    let path = Path::new("tasks.bin");
    
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(path)
      .expect("Couldn't access tasks.bin");

    let lock = Lock::new(Path::new("tasks.pid")).unwrap();
    
    let mut store = TaskStore {
      tasks: HashMap::new(),
      tasks_path: path.to_path_buf(),
      file_lock: lock
    };

    let meta: fs::Metadata = file.metadata().expect("Couldn't get file metadata");

    if meta.len() > 0 {
      let disk_store: DiskStore = decode_from(&mut file, bincode::SizeLimit::Infinite).unwrap();
      store.deserialize(disk_store);
      println!("Loaded {} tasks from disk", store.tasks.len());
    }

    store
  }

  fn deserialize(&mut self, store: DiskStore) {
    if store.version != 0 { panic!("Can't handle data with version {}", store.version) }
    
    self.tasks.clear();

    let tasks = store.tasks.into_iter().map(|t| (t.uuid, t));
    self.tasks.extend(tasks);
  }

  fn serialize(&self) -> DiskStore {
    let tasks = self.tasks.clone();
    DiskStore {
      version: 0,
      tasks: tasks.into_iter().map(|(_, task)| task).collect(),
    }
  }
}

impl Drop for TaskStore {
  fn drop(&mut self) {
    let file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open("tasks.bin").unwrap();
    let mut writer = BufWriter::new(file);

    encode_into(&self.serialize(), &mut writer, bincode::SizeLimit::Infinite).unwrap();
  }
}

use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};

#[derive(RustcEncodable, RustcDecodable)]
struct DiskStore {
  version: u32,
  tasks: Vec<Task>,
}
