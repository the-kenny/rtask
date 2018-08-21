extern crate clap;

use self::clap::{Arg, App, AppSettings, SubCommand};

use std::str::FromStr;

use task_ref::{TaskRef, TaskRefError};
use ::command::{Command, Flag};

fn flags_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("FLAG")
        .multiple(true)
        .validator(|arg| Flag::from_str(&arg)
                   .map(|_| ())
                   .ok_or(format!("Invalid Flag: {}", arg)))
}

fn task_id_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("TASK")
        .validator(|arg| TaskRef::from_str(&arg)
                   .map(|_| ())
                   .map_err(|err| format!("{}", err)))
}

fn app<'a, 'b>() -> App<'a, 'b> {
    App::new("rtask")
        .subcommand(SubCommand::with_name("show")
                    .arg(task_id_arg().multiple(true)))
        .subcommand(SubCommand::with_name("cancel")
                    .arg(task_id_arg().multiple(true)))
        .subcommand(SubCommand::with_name("done")
                    .arg(task_id_arg().multiple(true)))
        .subcommand(SubCommand::with_name("delete")
                    .arg(task_id_arg().multiple(true)))

        .subcommand(SubCommand::with_name("add")
                    .setting(AppSettings::AllowLeadingHyphen)
                    .setting(AppSettings::AllowMissingPositional)
                    .arg(Arg::with_name("TASK_DESCRIPTION")
                         .multiple(true)
                         .min_values(1)
                         .index(1)))
        .subcommand(SubCommand::with_name("list")
                    .setting(AppSettings::AllowLeadingHyphen)
                    .setting(AppSettings::AllowMissingPositional)
                    .arg(flags_arg().index(1)))
}

pub fn get_command() -> Result<Command, ::command::ParseError> {
    let matches = app().get_matches();
    debug!("args: {:?}", matches);

    match matches.subcommand() {
        ("", None) => Ok(Command::List(vec![])),
        ("list", args) => {
            let flags = args
                .and_then(|args| args.values_of("FLAG"))
                .map_or(vec![], |args| args.flat_map(Flag::from_str).collect());
            Ok(Command::List(flags))
        },
        ("show", Some(args)) => {
            let refs = args.values_of("TASK").expect("Couldn't get IDs")
                .map(TaskRef::from_str)
                .collect::<Result<Vec<TaskRef>, TaskRefError>>()?;

            Ok(Command::Show(refs))
        },
        ("add", args) => {
            let args: Vec<&str> = args
                .and_then(|args| args.values_of("TASK_DESCRIPTION"))
                .map(|args| args.collect())
                .unwrap_or(vec![]);

            // TODO: Only strip trailing and leading flags
            let flags: Vec<Flag> = args.iter().flat_map(|s| Flag::from_str(&s)).collect();
            let title: String = args.iter()
                .filter(|p| Flag::from_str(p).is_none()) // Ugh
                .fold(String::new(), |acc, arg| acc + " " + arg.as_ref())
                .trim()
                .to_string();

            if title != "" {
                debug!("title: {:?}, flags: {:?}", title, flags);
                Ok(Command::Add(title, flags))
            } else {
                Err(::command::ParseError("Failed to parse parameters".into()))
            }
        },

        command => unimplemented!("subcommand {:?}", command)
    }
}
