use std::collections::{BTreeMap, HashMap};
use task::*;
use task_ref::TaskRef;

use std::io;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    AddTask(Task),
    ChangeTaskTags {
        uuid: Uuid,
        added: Tags,
        removed: Tags,
    },
    ChangeTaskState(Uuid, TaskState),
    ChangeTaskPriority(Uuid, Priority),
    DeleteTask(Uuid),
    // Undo,
}

impl Effect {
    fn task_id<'a>(&'a self) -> &'a Uuid {
        use Effect::*;
        match *self {
            AddTask(Task { ref uuid, .. }) => uuid,
            ChangeTaskTags { ref uuid, .. } => uuid,
            ChangeTaskState(ref u, _) => u,
            ChangeTaskPriority(ref u, _) => u,
            DeleteTask(ref u) => u,
        }
    }

    pub fn print(&self, model: &Model, out: &mut io::Write) -> io::Result<()> {
        use Effect::*;

        let task = model.get_task(self.task_id()).unwrap(); // TODO

        match self {
            AddTask(_) => writeln!(out, "Added Task {}", task.short_id())?,
            DeleteTask(_) => writeln!(out, "Deleted task '{}'", task.description)?,
            ChangeTaskTags {
                ref added,
                ref removed,
                ..
            } => {
                if !added.is_empty() {
                    writeln!(out, "Added tags {:?}", added)?;
                }
                if !removed.is_empty() {
                    writeln!(out, "Removed tags {:?}", removed)?;
                }
            }
            ChangeTaskState(_uuid, ref state) => match *state {
                TaskState::Done(_) => writeln!(out, "Marking task '{}' as done", task.description)?,
                TaskState::Open => writeln!(out, "Marking task '{}' as open", task.description)?,
                TaskState::Canceled(_) => {
                    writeln!(out, "Marking task '{}' as canceled", task.description)?
                }
            },
            ChangeTaskPriority(_uuid, ref priority) => {
                writeln!(
                    out,
                    "Changed priority of task '{}' to {}",
                    task.description, priority
                )?;
            }
        };

        Ok(())
    }
}

pub type ScopeName = String;
pub type NumericalIds = HashMap<ScopeName, BTreeMap<u64, Uuid>>;

pub struct Model {
    // TODO: hide `tasks` and add `archived_tasks`
    pub tasks: HashMap<Uuid, Task>,
    pub applied_effects: Vec<Effect>,
    pub numerical_ids: NumericalIds,

    is_dirty: bool,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tasks: HashMap::new(),
            applied_effects: Vec::new(),
            numerical_ids: NumericalIds::new(),

            is_dirty: false,
        }
    }

    pub fn from_effects(effects: &[Effect]) -> Self {
        let mut model = Self::new();
        for effect in effects {
            model.apply_effect(&effect)
        }
        model.is_dirty = false;
        model
    }

    pub fn apply_effect(&mut self, effect: &Effect) -> () {
        use Effect::*;
        match effect.clone() {
            AddTask(task) => {
                self.add_task(task);
            }
            ChangeTaskTags {
                uuid,
                added,
                removed,
            } => {
                self.change_task_tags(&uuid, added, removed);
            }
            ChangeTaskState(uuid, state) => {
                self.change_task_state(&uuid, state);
            }
            ChangeTaskPriority(uuid, p) => {
                self.change_task_priority(&uuid, p);
            }
            DeleteTask(uuid) => {
                self.delete_task(&uuid);
            }
        }

        self.applied_effects.push(effect.clone());
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
        self.tasks.get_mut(u).expect("failed to get task").status = state;
    }

    fn change_task_priority(&mut self, u: &Uuid, priority: Priority) {
        self.tasks.get_mut(u).expect("failed to get task").priority = priority;
    }

    fn change_task_tags(&mut self, u: &Uuid, added: Tags, removed: Tags) {
        let ref mut tags = self.tasks.get_mut(u).expect("failed to get task").tags;

        for t in removed {
            tags.remove(&t);
        }
        for t in added {
            tags.insert(t);
        }
    }
}

// Numerical-ID Handling
impl Model {
    pub fn short_task_id(&self, scope_name: &str, task_id: &Uuid) -> Option<u64> {
        self.numerical_ids
            .get(scope_name)
            .and_then(|ids| ids.iter().find(|&(_, uuid)| uuid == task_id))
            .map(|(n, _)| *n)
    }

    pub fn recalculate_numerical_ids(&mut self, scope: &str, task_ids: &[Uuid]) {
        info!("Recalculating numerical-ids for scope {}", scope);

        self.is_dirty = true;

        let ids = task_ids
            .iter()
            .enumerate()
            .map(|(n, uuid)| ((n as u64) + 1, uuid.clone()))
            .collect();
        self.numerical_ids.insert(scope.into(), ids);
    }

    pub fn incremental_numerical_id(&mut self, scope: &str, task: &Uuid) -> u64 {
        debug!(
            "Calculating incremental numerical-id for {} in scope {}",
            task, scope
        );
        assert!(self.get_task(task).is_some());

        self.short_task_id(scope, task).unwrap_or_else(|| {
            self.is_dirty = true;
            let numerical_ids = self.numerical_ids
                .entry(scope.into())
                .or_insert(BTreeMap::new());

            let n = numerical_ids.iter().map(|(id, _)| *id).max().unwrap_or(0) + 1;

            numerical_ids.insert(n, task.clone());
            n
        })
    }
}

#[derive(Debug, PartialEq, Eq, Fail)]
pub enum FindTaskError {
    #[fail(display = "Couldn't find task")]
    TaskNotFound,
    #[fail(display = "Found multiple tasks")]
    MultipleResults,
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
        v.sort_by(|a, b| b.cmp(a));
        TaskIter { tasks: v, pos: 0 }
    }

    pub fn get_task<'a>(&'a self, uuid: &Uuid) -> Option<&'a Task> {
        self.tasks.get(uuid)
    }

    pub fn find_task<'a>(
        &'a self,
        scope_name: &str,
        task_ref: &TaskRef,
    ) -> Result<&'a Task, FindTaskError> {
        let uuids: Vec<&Uuid> = match *task_ref {
            TaskRef::FullUUID(ref u) => vec![u],
            TaskRef::ShortUUID(ref s) => self.tasks
                .keys()
                .filter(|uuid| uuid.simple().to_string().starts_with(s))
                .collect(),
            TaskRef::Numerical(ref n) => {
                match self.numerical_ids.get(scope_name).and_then(|x| x.get(n)) {
                    Some(uuid) => vec![uuid],
                    None => vec![],
                }
            }
        };

        use self::FindTaskError::*;
        match uuids.len() {
            0 => Err(TaskNotFound),
            1 => self.get_task(uuids[0])
                .map_or(Err(FindTaskError::TaskNotFound), Ok),
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
    use chrono;
    use std::str::FromStr;
    use uuid::Uuid;
    use {Priority, Task, TaskState};

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
        let s = TaskState::Done(chrono::Utc::now());
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
        assert_eq!(TaskRef::from_str("0"), Ok(TaskRef::Numerical(0)));
        assert!(TaskRef::from_str("-0").is_err());
    }

    #[test]
    fn test_short_uuid_ref() {
        for s in vec!["abcdef", "123abc", "000000"] {
            assert_eq!(TaskRef::from_str(s), Ok(TaskRef::ShortUUID(s.into())));
        }

        assert!(
            TaskRef::from_str("abcde").is_err(),
            "Short-UUID with len of 5"
        );
        assert!(
            TaskRef::from_str("abcdef1").is_err(),
            "Short-UUID with len of 7"
        );

        // Make sure that short-UUIDs are preferred
        assert_eq!(
            TaskRef::from_str("123456"),
            Ok(TaskRef::ShortUUID("123456".into()))
        );

        // non-base16 symbols
        assert!(TaskRef::from_str("rivers").is_err());
    }

    #[test]
    fn test_full_uuid_ref() {
        for _ in 1..100 {
            let uuid = Uuid::new_v4();
            assert_eq!(
                TaskRef::from_str(&uuid.hyphenated().to_string()),
                Ok(TaskRef::FullUUID(uuid))
            );
        }
    }

    #[test]
    fn test_incremental_numerical_id_empty_scope() {
        let mut m = Model::new();
        let t = Task::new("foo");
        let uuid = t.uuid.clone();
        m.add_task(t.clone());
        assert_eq!(m.incremental_numerical_id("defaut", &uuid), 1);
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
}
