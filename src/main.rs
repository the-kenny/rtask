extern crate rtask;

use rtask::task::*;
use rtask::store::*;
use rtask::commands::Command;

use std::env;
use std::fs;
use std::io::ErrorKind;

fn main() {
  chdir();
  
  let mut store = TaskStore::new();

  if let Some(command) = Command::from_args() {
    match command {
      Command::List => {
        for (_, ref task) in &store.tasks {
          println!("task (urgency: {}): {}", task.urgency(), task.description);
        }        
      },
      Command::Add(ref s) => {
        let task: Task = Task::new(s);
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
  dir.push(".rtasks");
  
  match fs::create_dir(&dir) {
    Err(ref err) if err.kind() == ErrorKind::AlreadyExists => (),
    Ok(_) => (),
    Err(_) => panic!("Couldn't create ~/.rtasks"),
  }
  
  env::set_current_dir(&dir).expect("Couldn't chdir");
}
