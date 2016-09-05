use ::task::*;
use ::TaskRef;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  ChangeTaskTags { uuid: Uuid, added: Tags, removed: Tags },
  ChangeTaskState(Uuid, TaskState),
  ChangeTaskPriority(Uuid, Priority),
  DeleteTask(Uuid),
  // Undo,
}

impl Effect {
  pub fn task_id<'a>(&'a self) -> Option<&'a Uuid> {
    use Effect::*;
    match *self {
      AddTask(_)                     => None,
      ChangeTaskTags{ ref uuid, .. } => Some(uuid),
      ChangeTaskState(ref u, _)      => Some(u),
      ChangeTaskPriority(ref u, _)   => Some(u),
      DeleteTask(ref u)              => Some(u),
    }
  }
}

pub struct Model {
  // TODO: hide `tasks` and add `archived_tasks`
  pub tasks: HashMap<Uuid, Task>,
  pub applied_effects: Vec<Effect>,
  pub numerical_ids: BTreeMap<Uuid, u64>,

  is_dirty: bool,
}

impl Model {
  pub fn new() -> Self {
    Model {
      tasks: HashMap::new(),
      applied_effects: Vec::new(),
      numerical_ids: BTreeMap::new(),

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
      AddTask(task)                          => { self.add_task(task); },
      ChangeTaskTags{ uuid, added, removed } => { self.change_task_tags(&uuid, added, removed); },
      ChangeTaskState(uuid, state)           => { self.change_task_state(&uuid, state); },
      ChangeTaskPriority(uuid, p)            => { self.change_task_priority(&uuid, p); },
      DeleteTask(uuid)                       => { self.delete_task(&uuid); },
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

  fn change_task_priority(&mut self, u: &Uuid, priority: Priority) {
    self.tasks.get_mut(u)
      .expect("failed to get task")
      .priority = priority;
  }

  fn change_task_tags(&mut self, u: &Uuid, added: Tags, removed: Tags) {
    let ref mut tags = self.tasks.get_mut(u)
      .expect("failed to get task")
      .tags;

    for t in removed { tags.remove(&t); };
    for t in added   { tags.insert(t);  };
  }

}

// Numerical-ID Handling
impl Model {
  pub fn recalculate_numerical_ids(&mut self, task_ids: &[Uuid]) {
    info!("Recalculating numerical-ids");

    self.is_dirty = true;

    self.numerical_ids = task_ids.iter()
      .enumerate()
      .map(|(n, uuid)| (uuid.clone(), (n as u64)+1))
      .collect();
  }
}

#[derive(Debug,PartialEq,Eq)]
pub enum FindTaskError {
  TaskNotFound,
  MultipleResults
}

impl Model {
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
      },
      TaskRef::Numerical(ref n) => {
        let res = self.numerical_ids.iter()
          .find(|&(_, i)| n == i);

        match res {
          Some((uuid, _)) => vec![uuid],
          None => vec![],
        }
      },
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

#[cfg(test)]
mod tests {
  use super::*;
  use ::{Task, TaskRef, TaskState, Priority};
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

  #[test]
  fn test_change_task_priority() {
    let mut m = Model::new();
    let t = Task::new("foo");
    let uuid = t.uuid.clone();
    m.add_task(t.clone());
    assert_eq!(m.tasks[&uuid].priority, Priority::Default);
    m.change_task_priority(&uuid, Priority::High);
    assert_eq!(m.tasks[&uuid].priority, Priority::High);
  }
}
