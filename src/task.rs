use std::collections::{HashMap, HashSet};
use chrono;
use uuid;

pub type Title    = String;
pub type Time     = chrono::DateTime<chrono::Utc>;
pub type Uuid     = uuid::Uuid;
pub type Tag      = String;
pub type Tags     = HashSet<Tag>;
pub type ExtraMap = HashMap<ExtraData, String>;

pub struct Age(chrono::Duration);

#[derive(Clone, Copy, Debug, PartialEq, Eq,
         Serialize, Deserialize)]
pub enum TaskState {
  Open,
  Done(Time),
  Canceled(Time),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize)]
pub enum Priority {
  Low,
  Default,
  High,
  // Custom(f32),
}

impl Default for Priority {
  fn default() -> Self { Priority::Default }
}

impl From<Priority> for f32 {
  fn from(o: Priority) -> Self {
    match o {
      Priority::Low     => -5.0,
      Priority::Default =>  0.0,
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
         Serialize, Deserialize)]
pub enum ExtraData {
  Notes = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
  pub description: Title,
  pub status:      TaskState,
  pub priority:    Priority,
  pub created:     Time,
  pub modified:    Time,
  pub uuid:        Uuid,
  pub tags:        Tags,
  pub extras:      ExtraMap,
}

impl Task {
  pub fn new(description: &str) -> Self {
    let now = chrono::Utc::now();
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
    let diff = chrono::Utc::now() - self.created;
    let seconds_per_day = chrono::Duration::days(1).num_seconds();
    let days = diff.num_seconds() as f32 / seconds_per_day as f32;

    let mut urgency = 0.0;
    urgency += days / 100.0; // Add 0.01 for every day since creation
    urgency += f32::from(self.priority); // Add priority
    urgency += self.tags.len() as f32 / 1000.0;

    urgency
  }

  pub fn age(&self) -> Age {
    Age(chrono::Utc::now() - self.created)
  }

  pub fn short_id(&self) -> String {
    let mut s = self.uuid.simple().to_string();
    s.truncate(6);
    s
  }

  pub fn is_open(&self) -> bool {
    match self.status {
      TaskState::Done(_)     => false,
      TaskState::Open        => true,
      TaskState::Canceled(_) => false,
    }
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

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum TagDirection {
  Added,
  Removed
}

const TAG_PREFIXES: &'static [ &'static str ] = &[ "+", "-" ];

use std::borrow::Cow;
pub trait StringExt {
  fn as_tag(&self) -> Option<(TagDirection, Tag)>;
  fn tag_name(&self) -> Option<Tag> { self.as_tag().map(|(_,name)| name) }
  fn tag_direction(&self) -> Option<TagDirection> { self.as_tag().map(|(dir,_)| dir) }

  fn as_added_tag(&self) -> Option<Tag> {
    self.as_tag().and_then(|t| match t {
      (TagDirection::Added, name) => Some(name),
      _ => None
    })
  }

  fn as_removed_tag(&self) -> Option<Tag> {
    self.as_tag().and_then(|t| match t {
      (TagDirection::Removed, name) => Some(name),
      _ => None
    })
  }

  fn ellipsize<'a>(&'a self, max_width: usize) -> Cow<'a, str>;
}

impl StringExt for str {
  fn as_tag(&self) -> Option<(TagDirection, Tag)> {
    TAG_PREFIXES.iter().find(|prefix| self.starts_with(&prefix[..])).and_then(|prefix| {
      let dir = match *prefix {
        "+" => Some(TagDirection::Added),
        "-" => Some(TagDirection::Removed),
        _   => None
      };

      dir.and_then(|dir| Some((dir, (self[prefix.len()..]).into())))
    })
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
    use chrono::Duration;

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
  fn test_as_tag() {
    assert_eq!("+foo".as_tag(), Some((TagDirection::Added, "foo".into())));
    assert_eq!("-foo".as_tag(), Some((TagDirection::Removed, "foo".into())));
    assert_eq!("+-foo".as_tag(), Some((TagDirection::Added, "-foo".into())));
    assert_eq!("-+foo".as_tag(), Some((TagDirection::Removed, "+foo".into())));
    assert_eq!("foo".as_tag(),  None);
  }

  #[test]
  fn test_tag_name() {
    assert_eq!("+foo".tag_name(), Some("foo".into()));
    assert_eq!("-foo".tag_name(), Some("foo".into()));
    assert_eq!("foo".tag_name(), None);
  }

  #[test]
  fn test_tag_direction() {
    assert_eq!("+foo".tag_direction(), Some(TagDirection::Added));
    assert_eq!("-foo".tag_direction(), Some(TagDirection::Removed));
    assert_eq!("+-foo".tag_direction(), Some(TagDirection::Added));
    assert_eq!("-+foo".tag_direction(), Some(TagDirection::Removed));
    assert_eq!("foo".tag_direction(),  None);
  }

  #[test]
  fn test_as_added_tag() {
    for (t, goal) in vec![("+foo", Some("foo".into())),
                          ("-foo", None),
                          ("+-foo", Some("-foo".into())),
                          ("-+foo", None),
                          ("foo", None)] {
      assert_eq!(t.as_added_tag(), goal);
    }
  }

  #[test]
  fn test_as_removed_tag() {
    for (t, goal) in vec![("-foo", Some("foo".into())),
                          ("+foo", None),
                          ("-+foo", Some("+foo".into())),
                          ("+-foo", None),
                          ("foo", None)] {
      assert_eq!(t.as_removed_tag(), goal);
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
