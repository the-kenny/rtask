extern crate rtask;

use std::env;
use rtask::task::*;
use rtask::store::*;
use std::fs;
use std::io::ErrorKind;

fn main() {
  chdir();
  
  let args: Vec<String> = env::args().skip(1).collect();
  let command = args.get(0).expect("Usage: rtask COMMAND");

  let store = TaskStore::new();

  match command.as_ref() {
    "list" => {
      for (_, ref task) in &store.tasks {
        println!("task (urgency: {}): {}", task.urgency(), task.description);
      }
    },
    "show" => println!("Showing single task..."),
    command => { println!("Invalid command: {}", command); return },
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
