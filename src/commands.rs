use ::task::*;
use std::env;

use std::iter::FromIterator;

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
  List,
  Show(Uuid),
  Add(Title, Tags),
}

impl Command {
  pub fn from_args() -> Option<Self> {
    let args: Vec<String> = env::args().skip(1).collect();
    Self::from_vec(&args)
  }

  fn from_vec(args: &Vec<String>) -> Option<Self> {
    if args.len() == 0 { return Some(Command::List) };

    // TODO: Simplify when slice_patterns get merged
    match args[0].as_ref() {
      "list" => {
        if args.len() > 1 { None } else { Some(Command::List) }
      },
      "show" if args.len() == 2 => {
        if let Ok(uuid) = Uuid::parse_str(&args[1]) {
          Some(Command::Show(uuid))
        } else { None }
      },
      "add" => {
        let params = args.iter().skip(1);

        let tags: Tags = Tags::from_iter(
          params.clone()
            .into_iter()
            .filter(|s| s.is_tag())
            .flat_map(|s| s.as_tag()));
        
        let title = params
          .filter(|p| !p.is_tag())
          .fold(String::new(), |acc, arg| acc + " " + arg);

        info!("title: {:?}, tags: {:?}", title, tags);

        Some(Command::Add(title.trim().to_string(), tags))
      },
      _ => None
    }
  }
}

#[test]
fn test_list() {
  let c = Command::from_vec(&vec!["list".to_string()]);
  assert_eq!(c, Some(Command::List));

  let c = Command::from_vec(&vec!["list".to_string(),
                                  "unimplemented_filter".to_string()]);
  assert_eq!(c, None);
}

#[test]
fn test_show() {
  let uuid = Uuid::new_v4();
  let c = Command::from_vec(&vec!["show".to_string(), uuid.hyphenated().to_string()]);
  assert_eq!(c, Some(Command::Show(uuid)));

  let c = Command::from_vec(&vec!["show".to_string()]);
  assert_eq!(c, None);

  let c = Command::from_vec(&vec!["show".to_string(), "asdfsafd".to_string()]);
  assert_eq!(c, None);
}


#[test]
fn test_add() {
  let c = Command::from_vec(&vec!["add".to_string(), "foo".to_string()]);
  assert_eq!(c, Some(Command::Add("foo".to_string(), Tags::new())));

  let c = Command::from_vec(&vec!["add".to_string(), "foo".to_string(), "bar".to_string()]);
  assert_eq!(c, Some(Command::Add("foo bar".to_string(), Tags::new())));
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
  assert_eq!(c, Some(Command::List));
}
