use time;
use uuid;

use std::collections::{HashMap, HashSet};

pub type Title = String;
pub type Time = time::Timespec;
pub type Uuid = uuid::Uuid;
pub type Tag = String;
pub type Tags = HashSet<Tag>;
pub type ExtraMap = HashMap<ExtraData, String>;

pub struct Age(time::Duration);

#[derive(Clone, Copy, Debug, PartialEq, Eq,
         RustcEncodable, RustcDecodable)]
pub enum TaskState {
  Open,
  Done(Time)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         RustcEncodable, RustcDecodable)]
pub enum Priority {
  Low,
  Default,
  High,
  // Custom(f32),
}

impl Default for Priority {
  fn default() -> Self { Priority::Default }
}

impl Into<f32> for Priority {
  fn into(self) -> f32 {
    match self {
      Priority::Low     => -5.0,
      Priority::Default =>   0.0,
      Priority::High    =>  5.0,
    }
  }
}

use std::str::FromStr;
impl FromStr for Priority {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().chars().next() {
      Some('l') => Ok(Priority::Low),
      Some('m') => Ok(Priority::Default),
      Some('d') => Ok(Priority::Default),
      Some('h') => Ok(Priority::High),
      _         => Err(()),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         RustcEncodable, RustcDecodable)]
pub enum ExtraData {
  Notes = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, RustcEncodable)]
pub struct Task {
  pub description: Title,
  pub status: TaskState,
  pub priority: Priority,
  pub created: Time,
  pub modified: Time,
  pub uuid: Uuid,
  pub tags: Tags,
  pub extras: ExtraMap,
}

impl Task {
  pub fn new(description: &str) -> Self {
    let now = time::get_time();
    Task {
      description: description.to_string(),
      status: TaskState::Open,
      priority: Priority::default(),
      created: now,
      modified: now,
      uuid: Uuid::new_v4(),
      tags: Tags::new(),
      extras: ExtraMap::new(),
    }
  }

  pub fn new_with_tags(description: &str, tags: Tags) -> Self {
    let mut t = Task::new(description);
    t.tags = tags;
    t
  }

  pub fn urgency(&self) -> f32 {
    let diff = time::get_time() - self.created;
    let days = diff.num_days();

    let mut urgency = 0.0;
    urgency += days as f32 / 100.0; // Add 0.01 for every day since creation
    urgency += self.priority.into(); // Add priority
    urgency
  }

  pub fn age(&self) -> Age {
    Age(time::get_time() - self.created)
  }

  pub fn short_id(&self) -> String {
    let mut s = self.uuid.simple().to_string();
    s.truncate(6);
    s
  }

  pub fn is_done(&self) -> bool {
    match self.status {
      TaskState::Done(_) => true,
      TaskState::Open    => false,
    }
  }

  // pub fn mark_done(&mut self) {
  //   use self::TaskState::*;
  //   match self.status {
  //     Open => self.status = Done(time::get_time()),
  //     _ => ()
  //   }
  // }
}

use rustc_serialize::{Decoder,Decodable};
impl Decodable for Task {
  fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
    let description = try!(d.read_struct_field("description", 0, Decodable::decode));
    let status      = try!(d.read_struct_field("status",      0, Decodable::decode));
    // Hack to allow missing fields (for backwards compatibility)
    let priority: Option<Priority> = try!(d.read_struct_field("priority",    0, Decodable::decode));
    let created     = try!(d.read_struct_field("created",     0, Decodable::decode));
    let modified    = try!(d.read_struct_field("modified",    0, Decodable::decode));
    let uuid        = try!(d.read_struct_field("uuid",        0, Decodable::decode));
    let tags        = try!(d.read_struct_field("tags",        0, Decodable::decode));
    let extras      = try!(d.read_struct_field("extras",      0, Decodable::decode));

    Ok(Task {
      description: description,
      status: status,
      priority: priority.unwrap_or(Priority::default()),
      created: created,
      modified: modified,
      uuid: uuid,
      tags: tags,
      extras: extras,
    })
  }
}

use std::cmp;
impl cmp::PartialOrd for Task {
  fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
    Some(self.cmp(&other))
  }
}

impl cmp::Ord for Task {
  fn cmp(&self, other: &Self) -> cmp::Ordering {
    let time_ord = other.created.cmp(&self.created);
    match self.urgency().partial_cmp(&other.urgency()) {
      None                       => time_ord,
      Some(cmp::Ordering::Equal) => time_ord,
      Some(v)                    => v,
    }
  }
}

const TAG_PREFIXES: &'static [ &'static str ] = &[ "t:", "tag:" ];

use std::borrow::Cow;
pub trait StringExt {
  fn is_tag(&self) -> bool;
  fn as_tag(&self) -> Option<Tag>;

  fn ellipsize<'a>(&'a self, max_width: usize) -> Cow<'a, str>;
}

impl StringExt for str {
  fn is_tag(&self) -> bool { TAG_PREFIXES.iter().any(|prefix| self.starts_with(prefix)) }
  fn as_tag(&self) -> Option<Tag> {
    if let Some(prefix) = TAG_PREFIXES.iter().find(|prefix| self.starts_with(&prefix[..])) {
      Some((self[prefix.len()..]).to_string())
    } else { None }
  }

  fn ellipsize<'a>(&'a self, max_width: usize) -> Cow<'a, str> {
    assert!(max_width > 0);

    let ellipsis = "...";

    if self.len() == 0 {
      self.into()
    } else if self.len() <= max_width {
      self.into()
    } else {
      let nchars = if max_width > ellipsis.len() {
        max_width - ellipsis.len()
      } else {
        max_width
      };
      let mut s: String = self.chars().take(nchars).collect();
      if nchars < max_width {
        s.push_str(ellipsis);
      }
      s.into()
    }
  }
}


use std::fmt;
impl fmt::Display for Age {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    let Age(d) = *self;
    let weeks  = d.num_weeks();
    let days   = d.num_days();
    let hours  = d.num_hours();
    let minutes = d.num_minutes();
    let seconds = d.num_seconds();

    let s = match (weeks,days,hours,minutes,seconds) {
      (0,0,0,0,n) => format!("{}s", n),
      (0,0,0,n,_) => format!("{}m", n),
      (0,0,n,_,_) => format!("{}h", n),
      (0,n,_,_,_) => format!("{}d", n),
      (n,_,_,_,_) => format!("{}w", n),
    };

    f.write_str(&s)
  }
}

impl fmt::Display for Priority {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    let s = match *self {
      Priority::Low     => "L",
      Priority::Default => "D",
      Priority::High    => "H",
    };
    f.write_str(s)
  }   
}

#[cfg(test)]
mod tests {
  use super::*;

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
    use time::Duration;

    let t = Task::new("old");
    let mut t2 = t.clone();
    assert_eq!(t.urgency(), t2.urgency());
    // Check if urgency increases when a job gets older
    t2.created = t2.created - Duration::days(2);
    assert!(t2.urgency() > t.urgency());
  }

  // #[test]
  // fn test_mark_done() {
  //   use TaskState::*;
  //   let mut t: Task = Task::new("foo");
  //   assert_eq!(Open, t.status);
  //   t.mark_done();
  //   match t.status {
  //     Done(_) => (),
  //     _ => assert!(false, "Task::mark_done() failed"),
  //   }
  // }

  #[test]
  fn test_is_tag_string() {
    let x = vec!["t:foo".to_string(),
                 "tag:foo".to_string()];

    for t in x {
      assert_eq!(true, t.is_tag());
      assert_eq!(Some("foo".to_string()), t.as_tag());
    }
  }

  #[test]
  fn test_ellipsize() {
    assert_eq!("foo".ellipsize(1), "f");
    assert_eq!("foo".ellipsize(2), "fo");
    assert_eq!("foo".ellipsize(3), "foo");
    assert_eq!("foo".ellipsize(100), "foo");
    assert_eq!("foobar".ellipsize(6), "foobar");
    assert_eq!("foobar 123".ellipsize(6), "foo...");
  }
}
