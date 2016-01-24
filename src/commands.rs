use ::task::Uuid;
use std::env;

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
  List,
  Show(Uuid),
}

impl Command {
  pub fn from_args() -> Option<Self> {
    let args: Vec<String> = env::args().skip(1).collect();
    Self::from_vec(&args)
  }

  fn from_vec(args: &Vec<String>) -> Option<Self> {
    if args.len() == 0 { return None };

    // TODO: Simplify when slice_patterns get merged
    match args[0].as_ref() {
      "list" => {
        if args.len() > 1 { None } else { Some(Command::List) }
      },
      "show" if args.len() == 2 => {
        if let Ok(uuid) = Uuid::parse_str(&args[1]) {
          Some(Command::Show(uuid))
        } else { None }
      }
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
  let c = Command::from_vec(&vec!["show".to_string(), uuid.to_hyphenated_string()]);
  assert_eq!(c, Some(Command::Show(uuid)));

  let c = Command::from_vec(&vec!["show".to_string()]);
  assert_eq!(c, None);

  let c = Command::from_vec(&vec!["show".to_string(), "asdfsafd".to_string()]);
  assert_eq!(c, None);
}
