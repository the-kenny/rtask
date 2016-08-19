use super::task::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  // AddTags(Uuid, Tags),
  // RemoveTags(Uuid, Tags),
  ChangeTaskState(Uuid, TaskState),
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
    use Effect::*;
    match effect.clone() {
      AddTask(task)                => { self.add_task(task); },
      DeleteTask(uuid)             => { self.delete_task(&uuid); },
      ChangeTaskState(uuid, state) => { self.change_task_state(&uuid, state); }
    }

    self.applied_effects.push(effect);
    self.is_dirty = true;
  }

  fn add_task(&mut self, t: Task) -> () {
    if self.tasks.insert(t.uuid, t).is_some() {
      panic!("UUID collision in Model::add_task");
    }
  }

  fn delete_task(&mut self, u: &Uuid) -> Option<Task> {
    self.tasks.remove(&u)
  }

  fn change_task_state(&mut self, u: &Uuid, state: TaskState) {
    self.tasks.get_mut(u)
      .expect("failed to get task")
      .status = state;
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
      1 => self.tasks.get(uuids[0]).map_or(Err(FindTaskError::TaskNotFound), Ok),
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

// TODO: Use references instead of ownership
#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef {
  ShortUUID(String),
  FullUUID(Uuid),
  // Numerical(u64),
}

#[derive(Debug)]
pub struct TaskRefError;

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

impl From<Uuid> for TaskRef {
  fn from(u: Uuid) -> Self {
    TaskRef::FullUUID(u)
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

#[cfg(test)]
mod tests {
  use super::*;
  use ::{Task, TaskState};
  use time;
  
  #[test]
  fn test_add_delete_task() {
    let mut m = Model::new();
    let t = Task::new("foo");
    let tref: TaskRef = t.uuid.clone().into();
    m.add_task(t.clone());
    assert_eq!(m.find_task(&tref), Ok(&t));
    assert_eq!(m.delete_task(&t.uuid), Some(t));
    assert_eq!(m.find_task(&tref), Err(FindTaskError::TaskNotFound));
  }

    #[test]
  fn test_change_task_task() {
    let mut m = Model::new();
    let t = Task::new("foo");
    let uuid = t.uuid.clone();
    m.add_task(t.clone());
    assert_eq!(m.tasks[&uuid].status, TaskState::Open);
    let s = TaskState::Done(time::get_time());
    m.change_task_state(&uuid, s);
    assert_eq!(m.tasks[&uuid].status, s);
  }
}
