extern crate clap;

use self::clap::{Arg, App, AppSettings, SubCommand};

use std::str::FromStr;

use task_ref::TaskRef;
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
                    .arg(Arg::with_name("TASK DESCRIPTION")
                         .multiple(true)))
        .subcommand(SubCommand::with_name("list")
                    .setting(AppSettings::AllowLeadingHyphen)
                    .setting(AppSettings::AllowMissingPositional)
                    .arg(flags_arg().index(1)))
}

pub fn get_command() -> Result<Command, ::command::ParseError> {
    let matches = app().get_matches();
    info!("args: {:?}", matches);

    match matches.subcommand() {
        ("", None) => Ok(Command::List(vec![])),
        ("list", args) => {
            let flags = args
                .and_then(|args| args.values_of("FLAG"))
                .map_or(vec![], |args| args.flat_map(Flag::from_str).collect());
            Ok(Command::List(flags))
        },
        command => unimplemented!("subcommand {:?}", command)
    }
}
