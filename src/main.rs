extern crate rtask;
extern crate env_logger;

use rtask::task::*;
use rtask::store::*;
use rtask::commands::Command;

use std::env;
use std::fs;
use std::io::ErrorKind;

fn main() {
  env_logger::init().unwrap();

  chdir();

  let mut store = TaskStore::new();

  if let Some(command) = Command::from_args() {
    match command {
      Command::List => {
        for (_, ref task) in &store.tasks {
          println!("{:<60} u:{:<3} id:{}", task.description.ellipsize(60), task.urgency(), task.uuid);
        }
      },
      Command::Add(title, tags) => {
        let task: Task = Task::new_with_tags(&title, tags);
        store.tasks.insert(task.uuid, task.clone());
        println!("Added task '{}'", task.description);
      },
      Command::Show(_) => unimplemented!(),
    }
  } else {
    panic!("Invalid command :-(")
  }
}

fn chdir() {
  let mut dir = env::home_dir().expect("Couldn't get home dir");
  dir.push(".rtasks/");

  match fs::create_dir(&dir) {
    Err(ref err) if err.kind() == ErrorKind::AlreadyExists => (),
    Ok(_) => (),
    Err(_) => panic!("Couldn't create ~/.rtasks"),
  }

  env::set_current_dir(&dir).expect("Couldn't chdir");
}
