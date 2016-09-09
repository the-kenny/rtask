use ::task::Uuid;

use regex::Regex;

// TODO: Use references instead of ownership
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::str::FromStr;
  use ::task::Uuid;

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
}
