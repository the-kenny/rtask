use ::task::*;
use ::task_ref::TaskRef;
use std::collections::{BTreeMap, HashMap};

type CustomTag = String;

#[derive(Clone, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  ChangeTaskTags { uuid: Uuid, added: Tags, removed: Tags },
  ChangeTaskState(Uuid, TaskState),
  ChangeTaskPriority(Uuid, Priority),
  DeleteTask(Uuid),
  CustomEffect(CustomTag, String),
}

impl Effect {
  // TODO: Do *all* effects have a related `TaskId`? If so, remove the
  // `Option`
  pub fn task_id<'a>(&'a self) -> Option<&'a Uuid> {
    use Effect::*;
    match *self {
      AddTask(Task{ ref uuid, .. })  => Some(uuid),
      ChangeTaskTags{ ref uuid, .. } => Some(uuid),
      ChangeTaskState(ref u, _)      => Some(u),
      ChangeTaskPriority(ref u, _)   => Some(u),
      DeleteTask(ref u)              => Some(u),
      CustomEffect(_, _)             => None,
    }
  }
}

pub type ScopeName = String;
pub type NumericalIds = HashMap<ScopeName, BTreeMap<u64, Uuid>>;

type CustomData = HashMap<String, String>;
type CustomHandler = Box<Fn(&Model, &mut CustomData, String) + Sync + 'static>;

pub struct Model {
  // TODO: hide `tasks` and add `archived_tasks`
  pub tasks: HashMap<Uuid, Task>,
  pub applied_effects: Vec<Effect>,
  pub numerical_ids: NumericalIds,
  pub custom_handlers: HashMap<&'static str, CustomHandler>,
  pub custom_data: HashMap<String, String>,

  is_dirty: bool,
}

impl Default for Model {
  fn default() -> Self {
    Model {
      tasks: HashMap::new(),
      applied_effects: Vec::new(),
      numerical_ids: NumericalIds::new(),

      custom_handlers: HashMap::new(),
      custom_data: HashMap::new(),

      is_dirty: false,
    }
  }
}

impl Model {
  pub fn new() -> Self {
    Model::default()
  }

  pub fn from_effects(effects: Vec<Effect>) -> Self {
    let mut model = Self::new();
    for effect in effects { model.apply_effect(effect) }
    model.is_dirty = false;
    model
  }

  pub fn apply_effect(&mut self, effect: Effect) {
    use Effect::*;
    match effect.clone() {
      AddTask(task)                          => { self.add_task(task); },
      ChangeTaskTags{ uuid, added, removed } => { self.change_task_tags(&uuid, added, removed); },
      ChangeTaskState(uuid, state)           => { self.change_task_state(&uuid, state); },
      ChangeTaskPriority(uuid, p)            => { self.change_task_priority(&uuid, p); },
      DeleteTask(uuid)                       => { self.delete_task(&uuid); },
      CustomEffect(tag, data)                => { self.handle_custom_effect(tag, data); },
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

  fn handle_custom_effect(&mut self, tag: String, data: String) {
    let handler = self.custom_handlers.get(&tag[..])
      .expect(&format!("No handler for custom tag: {}", tag));
    let mut custom_data = self.custom_data.clone();
    handler(&self, &mut custom_data, data);
    self.custom_data = custom_data;
  }
}

// Numerical-ID Handling
impl Model {
  pub fn short_task_id(&self, scope_name: &str, task_id: &Uuid) -> Option<u64> {
    self.numerical_ids.get(scope_name)
      .and_then(|ids| ids.iter().find(|&(_, uuid)| uuid == task_id))
      .map(|(n, _)| *n)
  }

  pub fn recalculate_numerical_ids(&mut self, scope: &str, task_ids: &[Uuid]) {
    info!("Recalculating numerical-ids for scope {}", scope);

    self.is_dirty = true;

    let ids = task_ids.iter()
      .enumerate()
      .map(|(n, uuid)| ((n as u64)+1, uuid.clone()))
      .collect();
    self.numerical_ids.insert(scope.into(), ids);
  }

  pub fn incremental_numerical_id(&mut self, scope: &str, task: &Uuid) -> u64 {
    debug!("Calculating incremental numerical-id for {} in scope {}", task, scope);
    assert!(self.get_task(task).is_some());

    self.short_task_id(scope, task).unwrap_or_else(|| {
      self.is_dirty = true;
      let mut numerical_ids = self.numerical_ids.entry(scope.into())
        .or_insert(BTreeMap::new());

      let n = numerical_ids.iter()
        .map(|(id,_)| *id)
        .max()
        .unwrap_or(0) + 1;

      numerical_ids.insert(n, task.clone());
      n
    })
  }
}

#[derive(Debug,PartialEq,Eq)]
pub enum FindTaskError {
  TaskNotFound,
  MultipleResults
}

pub struct TaskIter<'a> {
  tasks: Vec<&'a Task>,
  pos: usize,
}

impl<'a> Iterator for TaskIter<'a> {
  type Item = &'a Task;
  fn next(&mut self) -> Option<Self::Item> {
    let v = self.tasks.get(self.pos);
    self.pos += 1;
    v.map(|x| *x)
  }
}

impl Model {
  pub fn all_tasks<'a>(&'a self) -> TaskIter<'a> {
    let mut v: Vec<&Task> = self.tasks.values().collect();
    v.sort_by(|a,b| b.cmp(a));
    TaskIter {
      tasks: v,
      pos: 0,
    }
  }

  pub fn get_task<'a>(&'a self, uuid: &Uuid) -> Option<&'a Task> {
    self.tasks.get(uuid)
  }

  pub fn find_task<'a>(&'a self,
                       scope_name: &str,
                       task_ref: &TaskRef) -> Result<&'a Task, FindTaskError> {
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
        match self.numerical_ids.get(scope_name).and_then(|x| x.get(n)) {
          Some(uuid) => vec![uuid],
          None => vec![],
        }
      },
    };

    use self::FindTaskError::*;
    match uuids.len() {
      0 => Err(TaskNotFound),
      1 => self.get_task(uuids[0]).map_or(Err(FindTaskError::TaskNotFound), Ok),
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
  use uuid::Uuid;
  use std::str::FromStr;

  #[test]
  fn test_add_delete_task() {
    let mut m = Model::new();
    let t = Task::new("foo");
    m.add_task(t.clone());
    assert_eq!(m.get_task(&t.uuid), Some(&t));
    assert_eq!(m.delete_task(&t.uuid), Some(t.clone()));
    assert_eq!(m.get_task(&t.uuid), None);
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

  #[test]
  fn test_numerical_ref() {
    assert_eq!(TaskRef::from_str("42"), Ok(TaskRef::Numerical(42)));
    assert_eq!(TaskRef::from_str("0"),  Ok(TaskRef::Numerical(0)));
    assert!(TaskRef::from_str("-0").is_err());
  }

  #[test]
  fn test_short_uuid_ref() {
    for s in vec!["abcdef", "123abc", "000000"] {
      assert_eq!(TaskRef::from_str(s), Ok(TaskRef::ShortUUID(s.into())));
    }

    assert!(TaskRef::from_str("abcde").is_err(),   "Short-UUID with len of 5");
    assert!(TaskRef::from_str("abcdef1").is_err(), "Short-UUID with len of 7");

    // Make sure that short-UUIDs are preferred
    assert_eq!(TaskRef::from_str("123456"),
               Ok(TaskRef::ShortUUID("123456".into())));

    // non-base16 symbols
    assert!(TaskRef::from_str("rivers").is_err());
  }

  #[test]
  fn test_full_uuid_ref() {
    for _ in 1..100 {
      let uuid = Uuid::new_v4();
      assert_eq!(TaskRef::from_str(&uuid.hyphenated().to_string()),
                 Ok(TaskRef::FullUUID(uuid)));
    }
  }

  #[test]
  fn test_incremental_numerical_id_empty_scope() {
    let mut m = Model::new();
    let t = Task::new("foo");
    let uuid = t.uuid.clone();
    m.add_task(t.clone());
    assert_eq!(m.incremental_numerical_id("default", &uuid), 1);
  }

  #[test]
  #[should_panic]
  fn test_incremental_numerical_id_unknown_task() {
    let mut m = Model::new();
    m.incremental_numerical_id("default", &Uuid::new_v4());
  }

  #[test]
  fn test_incremental_numerical_id_already_exists() {
    let mut m = Model::new();
    let t = Task::new("foo");
    m.add_task(t.clone());
    m.recalculate_numerical_ids("default", &vec![t.uuid]);
    assert_eq!(m.incremental_numerical_id("default", &t.uuid), 1);
  }

  #[test]
  fn test_incremental_numerical_id() {
    let mut m = Model::new();
    let t = Task::new("foo");
    let t2 = Task::new("bar");
    m.add_task(t.clone());
    m.recalculate_numerical_ids("default", &vec![t.uuid]);
    m.add_task(t2.clone());
    assert_eq!(m.short_task_id("default", &t.uuid), Some(1));
    assert_eq!(m.incremental_numerical_id("default", &t2.uuid), 2);
    assert_eq!(m.short_task_id("default", &t2.uuid), Some(2));
  }

  #[test]
  fn test_custom_handlers_called() {
    use super::CustomData;
    let mut m = Model::new();

    fn handler(_model: &Model, store: &mut CustomData, data: String) {
      store.insert("test".into(), data);
    };

    m.custom_handlers.insert("test", Box::new(handler));
    m.apply_effect(Effect::CustomEffect("test".into(), "foo".into()));

    assert_eq!(m.custom_data.get("test".into()), Some(&"foo".into()));
  }
}
