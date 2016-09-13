use ::task::*;
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef {
  ShortUUID(String),
  FullUUID(Uuid),
  Numerical(u64),
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskRefError;

use std::fmt;
use std::str::FromStr;

impl FromStr for TaskRef {
  type Err = TaskRefError;
  fn from_str(s: &str) -> Result<TaskRef, TaskRefError> {
    lazy_static! {
      static ref SHORT_RE: Regex = Regex::new("^[0-9a-fA-F]{6}$").unwrap();
    }

    let numerical = u64::from_str(s).ok().map(TaskRef::Numerical);
    let uuid = Uuid::parse_str(s).ok().map(TaskRef::FullUUID);
    let short = if SHORT_RE.is_match(s) {
      Some(TaskRef::ShortUUID(s.into()))
    } else {
      None
    };

    uuid.or(short).or(numerical).map_or(Err(TaskRefError), Ok)
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
      TaskRef::FullUUID(ref u)  => f.write_str(&u.hyphenated().to_string()),
      TaskRef::Numerical(n)     => f.write_str(&n.to_string()),
    }
  }
}
