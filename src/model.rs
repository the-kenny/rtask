use super::task::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  // ChangeTaskState(Uuid, TaskState),
  // AddTags(Uuid, Tags),
  // RemoveTags(Uuid, Tags),
  // DeleteTask(Uuid),
  // Undo,
}

pub struct Model {
  pub tasks: HashMap<Uuid, Task>,
  pub applied_effects: Vec<Effect>,
}

impl Model {
  pub fn new() -> Self {
    Model {
      tasks: HashMap::new(),
      applied_effects: Vec::new(),
    }
  }

  pub fn from_effects(effects: Vec<Effect>) -> Self {
    let mut model = Self::new();
    for effect in effects { model.apply_effect(effect) }
    model
  }

  pub fn apply_effect(&mut self, effect: Effect) -> () {
    use Effect::*;
    self.applied_effects.push(effect.clone());

    match effect {
      AddTask(task) => self.add_task(task),
    }
  }

  fn add_task(&mut self, t: Task) -> () {
    if self.tasks.insert(t.uuid, t).is_some() {
      panic!("UUID collision in TaskStore::add_task");
    }
  }

  // TODO: Iterator
  pub fn all_tasks<'a>(&'a self) -> Vec<&'a Task> {
    let mut v: Vec<&Task> = self.tasks.values().collect();
    v.sort_by(|a,b| b.cmp(a));
    v
  }
}
