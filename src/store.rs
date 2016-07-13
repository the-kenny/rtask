use bincode;
use bincode::rustc_serialize::{decode_from, encode_into};
use std::fs;
use std::path::{Path,PathBuf};

use ::task::{Task, Uuid};
use ::file_lock::Lock;

use std::collections::{HashMap};

const CURRENT_VERSION: u32 = 1;

const PID_FILE: &'static str = "tasks.pid";

pub struct TaskStore {
  tasks: HashMap<Uuid, Task>,

  is_dirty: bool,
  tasks_path: PathBuf,
  _file_lock: Lock,
}

impl TaskStore {
  pub fn new() -> Self {
    Self::load_from("tasks.bin")
  }

  pub fn add_task(&mut self, t: &Task) -> () {
    let res = self.tasks.insert(t.uuid, t.clone());
    self.is_dirty = true;

    if res != None {
      panic!("UUID collision in TaskStore::add_task");
    }
  }

  pub fn all_tasks<'a>(&'a self) -> Vec<&'a Task> {
    let mut v: Vec<&Task> = self.tasks.values().collect();
    v.sort_by_key(|t| (t.urgency() * -1000.0) as i64);
    v
  }

  fn load_from<P: AsRef<Path>>(path: P) -> Self {
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&path)
      .expect("Couldn't access tasks.bin");

    let lock = Lock::new(Path::new(PID_FILE))
      .expect("Couldn't acquire lock");

    let mut store = TaskStore {
      tasks: HashMap::new(),
      tasks_path: path.as_ref().to_path_buf(),

      is_dirty: false,
      _file_lock: lock
    };

    let meta: fs::Metadata = file.metadata().expect("Couldn't get file metadata");

    if meta.len() > 0 {
      let disk_store = DiskStore::new_from(&mut file).unwrap();
      store.deserialize(disk_store);
      info!("Loaded {} tasks from disk", store.tasks.len());
    } else {
      // Nothing serialized yet.
      store.is_dirty = true;
    }

    store
  }

  fn deserialize(&mut self, store: DiskStore) {
    use std::iter::FromIterator;
    self.tasks = HashMap::from_iter(store.tasks.into_iter().map(|t| (t.uuid, t)));
  }

  fn serialize(&self) -> DiskStore {
    let tasks = self.tasks.clone();
    DiskStore {
      tasks: tasks.into_iter().map(|(_, task)| task).collect(),
    }
  }
}

impl Drop for TaskStore {
  fn drop(&mut self) {
    let mut file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&self.tasks_path)
      .expect("Failed to open tasks-file for writing");

    info!("Dropping TaskStore");
    if self.is_dirty {
      info!("Serializing jobs to disk");
      self.serialize().write(&mut file).unwrap();
    }
    fs::remove_file(PID_FILE)
      .expect("Failed to remove PID file");
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
    store.add_task(&task.clone());
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
  tasks: Vec<Task>,
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
    debug!("Got {} tasks in DiskStore", store.tasks.len());
    for t in &store.tasks { debug!("{:?}", t); }

    Ok(store)
  }
}
