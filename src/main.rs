extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;

use rtask::task::*;
use rtask::store::*;
use rtask::commands::Command;
use rtask::terminal_size::*;

use std::fs;
use std::io::ErrorKind;

fn main() {
  env_logger::init().unwrap();

  chdir();

  let mut store = TaskStore::new();

  if let Some(command) = Command::from_args() {
    match command {
      Command::List => {
        let right_padding = 10;
        let terminal_width = terminal_size().columns - right_padding;
        for task in store.all_tasks() {
          println!("{d:<w$} u:{urgency:<3}",
                   w=terminal_width,
                   d=task.description.ellipsize(60),
                   urgency=task.urgency());
        }
      },
      Command::Add(title, tags) => {
        let task: Task = Task::new_with_tags(&title, tags);
        store.add_task(&task);
        println!("Added task '{}'", task.description);
      },
      Command::Show(_) => unimplemented!(),
    }
  } else {
    panic!("Invalid command :-(")
  }
}

fn chdir() {
  use std::env;
  let dir = env::var("RTASK_DIRECTORY")
    .map(Into::into)
    .unwrap_or_else(|_| {
      let mut dir = env::home_dir().expect("Couldn't get home dir");
      dir.push(".rtasks/");
      dir
    })
    .canonicalize()
    .expect("Failed to get absolute path");

  info!("Working directory: {}", dir.display());

  match fs::create_dir(&dir) {
    Err(ref err) if err.kind() == ErrorKind::AlreadyExists => (),
    Ok(_) => (),
    Err(_) => panic!("Couldn't create ~/.rtasks"),
  }

  env::set_current_dir(&dir).expect("Couldn't chdir");
}
