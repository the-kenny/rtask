use super::task::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  // ChangeTaskState(Uuid, TaskState),
  // AddTags(Uuid, Tags),
  // RemoveTags(Uuid, Tags),
  DeleteTask(Uuid),
  // Undo,
}

pub struct Model {
  pub tasks: HashMap<Uuid, Task>,
  pub applied_effects: Vec<Effect>,

  is_dirty: bool,
}

impl Model {
  pub fn new() -> Self {
    Model {
      tasks: HashMap::new(),
      applied_effects: Vec::new(),
      is_dirty: false,
    }
  }

  pub fn from_effects(effects: Vec<Effect>) -> Self {
    let mut model = Self::new();
    for effect in effects { model.apply_effect(effect) }
    model.is_dirty = false;
    model
  }

  pub fn apply_effect(&mut self, effect: Effect) -> () {
    self.applied_effects.push(effect.clone());
    self.is_dirty = true;

    use Effect::*;
    match effect {
      AddTask(task)    => { self.add_task(task); },
      DeleteTask(uuid) => { self.delete_task(uuid); },
    }
  }

  fn add_task(&mut self, t: Task) -> () {
    if self.tasks.insert(t.uuid, t).is_some() {
      panic!("UUID collision in Model::add_task");
    }
  }

  fn delete_task(&mut self, u: Uuid) -> Option<Task> {
    self.tasks.remove(&u)
  }

  // TODO: Iterator
  pub fn all_tasks<'a>(&'a self) -> Vec<&'a Task> {
    let mut v: Vec<&Task> = self.tasks.values().collect();
    v.sort_by(|a,b| b.cmp(a));
    v
  }

  pub fn find_task<'a>(&'a self, task_ref: &TaskRef) -> Result<&'a Task, FindTaskError> {
    let uuids: Vec<&Uuid> = match *task_ref {
      TaskRef::FullUUID(ref u) => {
        vec![u]
      },
      TaskRef::ShortUUID(ref s) => {
        self.tasks.keys().filter(|uuid| {
          uuid.simple().to_string().starts_with(s)
        }).collect()
      }
    };

    use self::FindTaskError::*;
    match uuids.len() {
      0 => Err(TaskNotFound),
      1 => Ok(&self.tasks[uuids[0]]),
      _ => Err(MultipleResults),
    }
  }

  pub fn is_dirty(&self) -> bool {
    self.is_dirty
  }
}

#[derive(Debug,PartialEq,Eq)]
pub enum FindTaskError {
  TaskNotFound,
  MultipleResults
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef {
  ShortUUID(String),
  FullUUID(Uuid),
  // Numerical(u64),
}

const SHORT_UUID_MIN_LEN: usize = 6;

use std::fmt;
use std::str::FromStr;

impl FromStr for TaskRef {
  type Err = TaskRefError;
  fn from_str(s: &str) -> Result<TaskRef, TaskRefError> {
    let uuid = Uuid::parse_str(s).ok().map(TaskRef::FullUUID);
    let short = if s.len() >= SHORT_UUID_MIN_LEN {
      Some(TaskRef::ShortUUID(s.into()))
    } else {
      None
    };

    uuid.or(short).map_or(Err(TaskRefError), Ok)
  }
}

impl fmt::Display for TaskRef {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      TaskRef::ShortUUID(ref s) => f.write_str(s),
      TaskRef::FullUUID(ref u) => f.write_str(&u.hyphenated().to_string()),
    }
  }
}

#[derive(Debug)]
pub struct TaskRefError;
