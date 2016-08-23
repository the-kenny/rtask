use ::task::Uuid;

// TODO: Use references instead of ownership
#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef {
  ShortUUID(String),
  FullUUID(Uuid),
  // Numerical(u64),
}

#[derive(Debug)]
pub struct TaskRefError;

const SHORT_UUID_LEN: usize = 6;

use std::fmt;
use std::str::FromStr;

impl FromStr for TaskRef {
  type Err = TaskRefError;
  fn from_str(s: &str) -> Result<TaskRef, TaskRefError> {
    let uuid = Uuid::parse_str(s).ok().map(TaskRef::FullUUID);
    let short = if s.len() == SHORT_UUID_LEN {
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
