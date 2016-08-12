extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;

use rtask::store::*;
use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::fs;
use std::io::ErrorKind;

fn main() {
  env_logger::init().unwrap();

  chdir();

  let mut store = TaskStore::new();
  let mut model = &mut store.model;

  if let Some(command) = Command::from_args() {
    match command {
      Command::List => {
        // TODO: Calculate padding
        let right_padding = 10 + 8;
        let terminal_width = terminal_size().columns - right_padding;
        for task in model.all_tasks() {
          println!("{short} {d:<w$} u:{urgency:<3}",
                   short=task.short_id(),
                   w=terminal_width,
                   d=task.description.ellipsize(60),
                   urgency=task.urgency());
        }
      },
      Command::Add(title, tags) => {
        let task: Task = Task::new_with_tags(&title, tags);
        let desc = task.description.clone();
        model.apply_effect(Effect::AddTask(task));
        println!("Added task '{}'", desc);
      },
      Command::Show(s) => {
        // TODO: Try to parse `s` as complete UUID and (later) as
        // numerical short-id
        use rtask::FindTaskError::*;
        match model.find_task(&s) {
          Ok(task) => println!("{:?}", task),
          Err(TaskNotFound) => println!("No matching task found"),
          Err(MultipleResults) => println!("Found multiple tasks matching {}", s),
        }
      }
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
      dir.push(".rtask/");
      dir
    });
  fs::create_dir_all(&dir)
    .expect("Failed to create directory");
  
  let dir = dir.canonicalize()
    .expect("Failed to get absolute path");

  info!("Working directory: {}", dir.display());

  match fs::create_dir(&dir) {
    Err(ref err) if err.kind() == ErrorKind::AlreadyExists => (),
    Ok(_) => (),
    Err(e) => panic!("Couldn't create {}: {}", &dir.display(), e),
  }

  env::set_current_dir(&dir).expect("Couldn't chdir");
}
