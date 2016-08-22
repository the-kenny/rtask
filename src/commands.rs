use ::task::*;
use ::model::{TaskRef, TaskRefError};

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
  List,
  Show(TaskRef),
  Add(Title, Tags),
  MarkDone(TaskRef),
  Delete(TaskRef),
}

impl Command {
  pub fn from_args() -> Result<Self, ParseError> {
    let args: Vec<String> = env::args().skip(1).collect();
    Self::from_vec(&args)
  }

  fn from_vec(args: &Vec<String>) -> Result<Self, ParseError> {
    if args.len() == 0 {
      return Ok(Command::List)
    }

    // Try to parse args[0] as TaskRef first
    if let Ok(tr) = TaskRef::from_str(&args[0]) {
      match args.get(1).map(|s| &s[..]) {
        None => Ok(Command::Show(tr)),
        Some("done") => Ok(Command::MarkDone(tr)),
        _ => unimplemented!()
      }
    } else {
      // TODO: Simplify when slice_patterns get stabilized
      match args[0].as_ref() {
        "list" if args.len() == 1 => {
          if args.len() > 1 {
            Err(ParseError("Too many arguments to `list`".into()))
          } else {
            Ok(Command::List)
          }
        },
        "list" => Err(ParseError("Invalid arguments".into())),
        "show" if args.len() == 2 => {
          try!(TaskRef::from_str(&args[1]).map(Command::Show).map(Ok))
        },
        "show" => Err(ParseError("Invalid arguments".into())),
        "add" => {
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
            info!("title: {:?}, tags: {:?}", title, tags);

            Ok(Command::Add(title, tags))
          } else {
            Err(ParseError("Failed to parse parameters".into()))
          }
        },
        "del" if args.len() == 2 => {
          try!(TaskRef::from_str(&args[1]).map(Command::Delete).map(Ok))
        },
        "del" => Err(ParseError("Invalid arguments".into())),
        v => Err(ParseError(format!("Unknown command {}", v)))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ::task::*;

  #[test]
  fn test_list() {
    let c = Command::from_vec(&vec!["list".to_string()]);
    assert_eq!(c, Ok(Command::List));

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
    assert_eq!(c, Ok(Command::List));
  }
}
