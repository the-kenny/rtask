use bincode;
use bincode::rustc_serialize::{decode_from, encode_into};
use std::fs;
use std::path::{Path,PathBuf};
use std::io::{BufWriter};

use ::task::{Task, Uuid};
use ::file_lock::Lock;

use std::collections::{HashMap};

const PID_FILE: &'static str = "tasks.pid";

pub struct TaskStore {
  tasks: HashMap<Uuid, Task>,

  tasks_path: PathBuf,
  _file_lock: Lock,
}

impl TaskStore {
  pub fn new() -> Self {
    Self::load_from("tasks.bin")
  }

  pub fn add_task(&mut self, t: &Task) -> () {
    let res = self.tasks.insert(t.uuid, t.clone());
    if res != None {
      panic!("UUID collision in TaskStore::add_task");
    }
  }

  fn load_from<P: AsRef<Path>>(path: P) -> Self {
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&path)
      .expect("Couldn't access tasks.bin");

    let lock = Lock::new(Path::new(PID_FILE)).unwrap();

    let mut store = TaskStore {
      tasks: HashMap::new(),
      tasks_path: path.as_ref().to_path_buf(),
      _file_lock: lock
    };

    let meta: fs::Metadata = file.metadata().expect("Couldn't get file metadata");

    if meta.len() > 0 {
      let disk_store: DiskStore = decode_from(&mut file, bincode::SizeLimit::Infinite).unwrap();
      store.deserialize(disk_store);
      info!("Loaded {} tasks from disk", store.tasks.len());
    }

    store
  }

  fn deserialize(&mut self, store: DiskStore) {
    if store.version != 0 {
      panic!("Can't handle data with version {}", store.version)
    }

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
      .open(&self.tasks_path).unwrap();
    let mut writer = BufWriter::new(file);

    info!("Dropping TaskStore, serializing jobs to disk");

    encode_into(&self.serialize(), &mut writer, bincode::SizeLimit::Infinite).unwrap();
    fs::remove_file(PID_FILE).unwrap();
  }
}

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
    let mut store = TaskStore::load_from(&tempfile);
    assert_eq!(0, store.tasks.len());
    store.tasks.insert(task.uuid, task.clone());
    assert_eq!(1, store.tasks.len());
    // store drops, gets serialized
  }
  {
    // Load from file, check if everything is as we've left it
    let store = TaskStore::load_from(&tempfile);
    assert_eq!(1, store.tasks.len());
    assert_eq!(Some(&task), store.tasks.get(&task.uuid));
  }

  fs::remove_file(tempfile).unwrap();
}

// On-Disk representation
#[derive(RustcEncodable, RustcDecodable)]
struct DiskStore {
  version: u32,
  tasks: Vec<Task>,
}
