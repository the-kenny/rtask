use ::task::*;
use ::{TaskRef, TaskRefError};

use std::env;
use std::iter::FromIterator;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl From<TaskRefError> for ParseError {
  fn from(_e: TaskRefError) -> Self {
    ParseError("Failed to parse task-id".into())
  }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
  List(Tags),
  Show(TaskRef),
  Add(Title, Tags),
  MarkDone(TaskRef),
  Delete(TaskRef),
  ChangePriority(TaskRef, Priority),
  ChangeTags{ task_ref: TaskRef, added: Tags, removed: Tags},
}

impl Command {
  pub fn from_args() -> Result<Self, ParseError> {
    let args: Vec<String> = env::args().skip(1).collect();
    Self::from_vec(&args)
  }

  pub fn task_ref<'a>(&'a self) -> Option<&'a TaskRef> {
    use self::Command::*;
    match *self {
      Show(ref r)                => Some(r),
      MarkDone(ref r)            => Some(r),
      Delete(ref r)              => Some(r),
      ChangePriority(ref r, _)   => Some(r),
      _                          => None
    }
  }

  pub fn should_recalculate_ids(&self) -> bool {
    match *self {
      Command::List(_) => true,
      _ => false
    }
  }

  fn from_vec(args: &Vec<String>) -> Result<Self, ParseError> {
    // Try to parse args[0] as TaskRef first
    if let Some(tr) = args.get(0).and_then(|s| TaskRef::from_str(s).ok()) {
      match args.get(1).map(|s| s.as_ref()) {
        None             => Ok(Command::Show(tr)),
        Some("show")     => Ok(Command::Show(tr)),
        Some("done")     => Ok(Command::MarkDone(tr)),
        Some("delete")   => Ok(Command::Delete(tr)),
        Some("priority") => {
          if let Some(priority) = args.get(2).and_then(|s| Priority::from_str(&s).ok()) {
            Ok(Command::ChangePriority(tr, priority))
          } else {
            Err(ParseError("Failed to parse priority".into()))
          }
        },
        Some("tag") => {
          let mut added   = Tags::new();
          let mut removed = Tags::new();
          for mut t in args.iter().skip(2).cloned() {
            if t.starts_with("+") {
              t.remove(0); added.insert(t);
            } else if t.starts_with("-") {
              t.remove(0); removed.insert(t);
            } else {
              return Err(ParseError(format!("Usage: <task-ref> tag +foo -bar")))
            }
          }

          Ok(Command::ChangeTags{
            task_ref: tr,
            added: added,
            removed: removed,
          })
        }
        Some(cmd) => Err(ParseError(format!("Invalid command '{}'", cmd)))
      }
    } else {
      match args.get(0).map(|s| s.as_ref()) {
        None | Some("list") => {
          let tags: Option<Tags> = args.iter().skip(1).map(|s| s.as_tag()).collect();
          println!("tags: {:?}", tags);
          match tags {
            Some(tags) => Ok(Command::List(tags)),
            None       => Err(ParseError(format!("Invalid arguments"))),
          }

        },
        Some("add") => {
          let params = args.iter().skip(1);

          let tags: Tags = Tags::from_iter(
            params.clone()
              .into_iter()
              .filter(|s| s.is_tag())
              .flat_map(|s| s.as_tag()));

          let title = params
            .filter(|p| !p.is_tag())
            .fold(String::new(), |acc, arg| acc + " " + arg)
            .trim()
            .to_string();

          if title != "" {
            debug!("title: {:?}, tags: {:?}", title, tags);

            Ok(Command::Add(title, tags))
          } else {
            Err(ParseError("Failed to parse parameters".into()))
          }
        },
        Some(v) => Err(ParseError(format!("Unknown command {}", v)))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::task::*;
  use std::iter::FromIterator;

  #[test]
  fn test_list() {
    let c = Command::from_vec(&vec!["list".to_string()]);
    assert_eq!(c, Ok(Command::List(Tags::new())));

    let c = Command::from_vec(&vec!["list".to_string(),
                                    "tag:foo".to_string()]);
    assert_eq!(c, Ok(Command::List(Tags::from_iter(vec!["foo".into()]))));

    let c = Command::from_vec(&vec!["list".to_string(),
                                    "unimplemented_filter".to_string()]);
    assert!(c.is_err());
  }

  #[test]
  #[ignore]
  fn test_show() {
    // let c = Command::from_vec(&vec!["show".to_string(), "foo".to_string()]);
    // assert_eq!(c, Some(Command::Show("foo".to_string())));

    // let c = Command::from_vec(&vec!["show".to_string()]);
    // assert_eq!(c, None);

    // let c = Command::from_vec(&vec!["show".to_string(), "asdfsafd".to_string()]);
    // assert_eq!(c, Some(Command::Show("asdfsafd".to_string())));
  }


  #[test]
  fn test_add() {
    let c = Command::from_vec(&vec!["add".to_string(), "foo".to_string()]);
    assert_eq!(c, Ok(Command::Add("foo".to_string(), Tags::new())));

    let c = Command::from_vec(&vec!["add".to_string(), "foo".to_string(), "bar".to_string()]);
    assert_eq!(c, Ok(Command::Add("foo bar".to_string(), Tags::new())));
  }

  #[test]
  fn test_tag_semantics() {
    let params = vec!["add".to_string(),
                      "tag:foo".to_string(),
                      "my title containing tag:42".to_string(),
                      "t:42 foo".to_string()];
    if let Command::Add(title, tags) = Command::from_vec(&params).unwrap() {
      assert_eq!(title, "my title containing tag:42");
      assert!(tags.contains("42 foo"));
      assert!(tags.contains("foo"));
    } else {
      assert!(false, "Command parsing failed");
    }
  }

  #[test]
  fn test_default() {
    let c = Command::from_vec(&vec![]);
    assert_eq!(c, Ok(Command::List(Tags::new())));
  }
}
