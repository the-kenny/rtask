use time;
use uuid;

use std::collections::{HashSet};

pub type Title = String;
pub type Time = time::Timespec;
pub type Uuid = uuid::Uuid;
pub type Tags = HashSet<String>;

#[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq)]
pub enum TaskState {
  Open,
  Done(Time)
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq)]
pub struct Task {
  pub description: Title,
  pub status: TaskState,
  pub created: Time,
  pub modified: Time,
  pub uuid: Uuid,
  pub tags: Tags,
}

impl Task {
  pub fn new(description: &str) -> Self {
    let now = time::get_time();
    Task {
      description: description.to_string(),
      status: TaskState::Open,
      created: now,
      modified: now,
      uuid: Uuid::new_v4(),
      tags: Tags::new(),
    }
  }

  pub fn urgency(&self) -> f32 {
    let diff = self.created - time::get_time();
    let days = diff.num_days();

    let mut urgency = 0.0;

    // Add 0.01 for every day since creation
    urgency = urgency + (days as f32 / 100.0);
    
    urgency
  }

  pub fn mark_done(&mut self) {
    use self::TaskState::*;
    match self.status {
      Open => self.status = Done(time::get_time()),
      _ => ()
    }
  }
}

#[test]
fn test_creation() {
  let t = Task::new("foo");
  assert_eq!(&t.description, "foo");
  assert_eq!(t.status, TaskState::Open);
  assert_eq!(t.tags, Tags::new());
  assert_eq!(false, t.uuid.is_nil());
}

#[test]
fn test_mark_done() {
  use self::TaskState::*;
  let mut t: Task = Task::new("foo");
  assert_eq!(Open, t.status);
  t.mark_done();
  match t.status {
    Done(_) => (),
    _ => panic!("Task::mark_done() failed"),
  }
}
