extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;

use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::fs;
use std::io::ErrorKind;

fn main() {
  env_logger::init().unwrap();

  chdir();

  let mut store = Storage::new().expect("Failed to open store");
  let model = store.model();

  let command = Command::from_args();

  if let Err(e) = command {
    println!("Error while parsing command: {}", e.0);
    return;
  } else if let Ok(command) = command {
    info!("Command: {:?}", command);

    use rtask::FindTaskError::*;

    let effect = match command {
      Command::List => {
        // TODO: Calculate padding
        let right_padding = 10 + 8;
        let terminal_width = terminal_size().columns - right_padding;
        for task in model.all_tasks()
          .iter()
          .filter(|t| t.status == TaskState::Open) {
          println!("{short} {d:<w$} u:{urgency:<3}",
                   short=task.short_id(),
                   w=terminal_width,
                   d=task.description.ellipsize(60),
                   urgency=task.urgency());
        }

        None
      },
      Command::Show(r) => {
        match model.find_task(&r) {
          Ok(task) => println!("{:?}", task),
          Err(TaskNotFound) => println!("Found no tasks matching {}", r),
          Err(MultipleResults) => println!("Found multiple tasks matching {}", r),
        }

        None
      },
      Command::Add(title, tags) => {
        let task: Task = Task::new_with_tags(&title, tags);
        println!("Adding task '{}'", task.description);
        Some(Effect::AddTask(task))
      },
      Command::Delete(r) => {
        match model.find_task(&r) {
          Ok(task) => {
            println!("Deleting task '{}'", task.description);
            Some(Effect::DeleteTask(task.uuid.clone()))
          },
          Err(TaskNotFound) => {
            println!("No matching task found");
            None
          },
          Err(MultipleResults) => {
            println!("Found multiple tasks matching {}", r);
            None
          }
        }
      }
    };

    effect.map(|e| model.apply_effect(e));
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
