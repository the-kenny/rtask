use task::*;
use task_ref::{TaskRef, TaskRefError, TaskRefs};

use std::str::FromStr;
use std::{env, fmt};

mod flag;
pub use self::flag::*;

// TODO: Use a proper enum
#[derive(Debug, PartialEq, Eq, Fail)]
#[fail(display = "{}", _0)]
pub struct ParseError(pub String);

impl From<TaskRefError> for ParseError {
    fn from(_e: TaskRefError) -> Self {
        ParseError("Failed to parse task-id".into())
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    List(Vec<Flag>),
    Show(TaskRefs),
    Add(Title, Vec<Flag>),
    MarkDone(TaskRefs),
    MarkCanceled(TaskRefs),
    Delete(TaskRefs),
    // This Command is used to apply multiple state changes coming from
    // a set of CLI flags (`Flag`)
    ChangeTaskProperties {
        task_refs: TaskRefs,
        added_tags: Tags,
        removed_tags: Tags,
        priority: Option<Priority>,
    },
}

impl Command {
    pub fn from_args() -> Result<Self, ParseError> {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::from_slice(&args)
    }

    fn from_slice<S: fmt::Debug + AsRef<str>>(args: &[S]) -> Result<Self, ParseError> {
        let task_refs = args.iter()
            .take_while(|s| TaskRef::from_str(s.as_ref()).is_ok())
            .map(|s| TaskRef::from_str(s.as_ref()).unwrap())
            .collect::<Vec<TaskRef>>();

        debug!("Got task_refs: {:?}", task_refs);

        let args = args.into_iter().skip(task_refs.len()).collect::<Vec<_>>();

        if !task_refs.is_empty() {
            match args.get(0).map(|s| s.as_ref()) {
                None => Ok(Command::Show(task_refs)),
                Some("show") => Ok(Command::Show(task_refs)),
                Some("done") => Ok(Command::MarkDone(task_refs)),
                Some("cancel") => Ok(Command::MarkCanceled(task_refs)),
                Some("delete") => Ok(Command::Delete(task_refs)),
                Some("edit") => {
                    // Parse 'args' as flags
                    let flags = args.iter()
                        .skip(1)
                        .map(Flag::from_str)
                        .collect::<Vec<Option<Flag>>>();

                    if !flags.is_empty() {
                        let mut added = Tags::new();
                        let mut removed = Tags::new();
                        let mut priority = None;

                        info!("Got flags for tasks {:?}: {:?}", task_refs, flags);
                        for flag in flags.into_iter() {
                            match flag {
                                None => {
                                    return Err(ParseError(format!("Invalid flag set '{:?}'", args)))
                                }
                                Some(Flag::Priority(p)) => priority = Some(p),
                                Some(Flag::TagPositive(t)) => {
                                    added.insert(t);
                                }
                                Some(Flag::TagNegative(t)) => {
                                    removed.insert(t);
                                }
                            }
                        }

                        Ok(Command::ChangeTaskProperties {
                            task_refs: task_refs,
                            added_tags: added,
                            removed_tags: removed,
                            priority: priority,
                        })
                    } else {
                        Err(ParseError(format!(
                            "Got no changes for task(s) {:?}",
                            task_refs
                        )))
                    }
                }
                Some(cmd) => Err(ParseError(format!("Unknown command {}", cmd))),
            }
        } else {
            match args.get(0).map(|s| s.as_ref()) {
                Some("add") => {
                    // TODO: Get rid of all this pesky cloning
                    let params = &args[1..];

                    let flags = params.iter().flat_map(|s| Flag::from_str(&s)).collect();

                    let title = params.iter()
            .filter(|p| Flag::from_str(p).is_none()) // Ugh
            .fold(String::new(), |acc, arg| acc + " " + arg.as_ref())
            .trim()
            .to_string();

                    if title != "" {
                        debug!("title: {:?}, flags: {:?}", title, flags);

                        Ok(Command::Add(title, flags))
                    } else {
                        Err(ParseError("Failed to parse parameters".into()))
                    }
                }
                None | Some("list") => (if args.get(0).map(|s| s.as_ref()) == Some("list") {
                    &args[1..]
                } else {
                    &args[..]
                }).iter()
                    .map(Flag::from_str)
                    .collect::<Option<Vec<Flag>>>()
                    .map(Command::List)
                    .ok_or_else(|| ParseError("Found invalid flags".into())),
                _ => panic!("Unknown command {:?}", args[0]),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list() {
        let c = Command::from_slice(&["list"]);
        assert_eq!(c, Ok(Command::List(Default::default())));

        let c = Command::from_slice(&["list", "+foo"]);
        assert_eq!(c, Ok(Command::List(vec![Flag::TagPositive("foo".into())])));

        let c = Command::from_slice(&["list", "-foo"]);
        assert_eq!(c, Ok(Command::List(vec![Flag::TagNegative("foo".into())])));

        let c = Command::from_slice(&["list", "+foo", "-bar", "p:h"]);
        assert_eq!(
            c,
            Ok(Command::List(vec![
                Flag::TagPositive("foo".into()),
                Flag::TagNegative("bar".into()),
                Flag::Priority(Priority::High),
            ]))
        );

        assert!(Command::from_slice(&["list", "unimplemented"]).is_err());
    }

    #[test]
    #[ignore]
    fn test_show() {
        unimplemented!()
        // let c = Command::from_slice(&vec!["show", "foo"]);
        // assert_eq!(c, Some(Command::Show("foo".into())));

        // let c = Command::from_slice(&vec!["show"]);
        // assert_eq!(c, None);

        // let c = Command::from_slice(&vec!["show", "asdfsafd"]);
        // assert_eq!(c, Some(Command::Show("asdfsafd".into())));
    }

    #[test]
    fn test_add() {
        let c = Command::from_slice(&vec!["add", "foo"]);
        assert_eq!(c, Ok(Command::Add("foo".into(), vec![])));

        let c = Command::from_slice(&vec!["add", "foo", "bar"]);
        assert_eq!(c, Ok(Command::Add("foo bar".into(), vec![])));
    }

    #[test]
    fn test_tag_flag_semantics() {
        let params = vec![
            "add",
            "+foo",
            "my title containing +42",
            "priority:h",
            "+42 foo",
        ];
        if let Command::Add(title, flags) = Command::from_slice(&params).unwrap() {
            assert_eq!(title, "my title containing +42");
            assert_eq!(
                flags,
                vec![
                    Flag::TagPositive("foo".into()),
                    Flag::Priority(Priority::High),
                    Flag::TagPositive("42 foo".into()),
                ]
            );
        } else {
            assert!(false, "Command parsing failed");
        }
    }

    #[test]
    fn test_default() {
        let empty: [&'static str; 0] = [];
        let c = Command::from_slice(&empty);
        assert_eq!(c, Ok(Command::List(Default::default())));
    }
}
