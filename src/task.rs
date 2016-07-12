use time;
use uuid;

use std::collections::{HashSet};

pub type Title = String;
pub type Time = time::Timespec;
pub type Uuid = uuid::Uuid;
pub type Tag = String;
pub type Tags = HashSet<Tag>;

#[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq, Eq)]
pub enum TaskState {
  Open,
  Done(Time)
}

#[derive(RustcEncodable, RustcDecodable, Clone, Debug, PartialEq, Eq)]
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

  pub fn new_with_tags(description: &str, tags: Tags) -> Self {
    let now = time::get_time();
    Task {
      description: description.to_string(),
      status: TaskState::Open,
      created: now,
      modified: now,
      uuid: Uuid::new_v4(),
      tags: tags,
    }
  }

  pub fn urgency(&self) -> f32 {
    let diff = time::get_time() - self.created;
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

// TODO: Implement `Ord` on Task by using `urgency()` &
// `created/odified`
use std::cmp;
impl cmp::PartialOrd for Task {
  fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
    self.urgency().partial_cmp(&other.urgency())
  }
}

#[test]
fn test_creation() {
  let t = Task::new("foo");
  assert_eq!(&t.description, "foo");
  assert_eq!(t.status, TaskState::Open);
  assert_eq!(t.tags, Tags::new());
  assert_eq!(false, t.uuid.is_nil());

  let mut tags = Tags::new();
  tags.insert("some-tag".to_string());
  let t = Task::new_with_tags("foo", tags.clone());
  assert_eq!(&t.description, "foo");
  assert_eq!(t.tags, tags);

}

#[test]
fn test_urgency() {
  let t = Task::new("old");
  let mut t2 = t.clone();
  assert_eq!(t.urgency(), t2.urgency());
  // Check if urgency increases when a job gets older
  t2.created = t2.created - time::Duration::days(2);
  assert!(t2.urgency() > t.urgency());
}

#[test]
fn test_mark_done() {
  use self::TaskState::*;
  let mut t: Task = Task::new("foo");
  assert_eq!(Open, t.status);
  t.mark_done();
  match t.status {
    Done(_) => (),
    _ => assert!(false, "Task::mark_done() failed"),
  }
}

const TAG_PREFIXES: &'static [ &'static str ] = &[ "t:", "tag:" ];

pub trait StringExt {
  fn is_tag(&self) -> bool;
  fn as_tag(&self) -> Option<Tag>;

  fn ellipsize(&self, max_width: usize) -> String;
}

impl StringExt for String {
  fn is_tag(&self) -> bool { TAG_PREFIXES.iter().any(|prefix| self.starts_with(prefix)) }
  fn as_tag(&self) -> Option<Tag> {
    if let Some(prefix) = TAG_PREFIXES.iter().find(|prefix| self.starts_with(&prefix[..])) {
      Some((self[prefix.len()..]).to_string())
    } else { None }
  }

  fn ellipsize(&self, max_width: usize) -> String {
    let ellipsis = "...";
    let cut = max_width - (ellipsis.len());
    if self.len() <= cut {
      self.clone()
    } else {
      let mut s = self.trim_right()[0..cut].to_string();
      s.push_str(ellipsis);
      s
    }
  }
}

#[test]
fn test_is_tag_string() {
  let x = vec!["t:foo".to_string(),
               "tag:foo".to_string()];

  for t in x {
    assert_eq!(true, t.is_tag());
    assert_eq!(Some("foo".to_string()), t.as_tag());
  }
}
