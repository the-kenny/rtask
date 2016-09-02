extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate time;

use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::{fs,mem};
use std::io::ErrorKind;

pub const PID_FILE: &'static str = "tasks.pid";

#[derive(Debug)]
enum HandleCommandError {
  FindTaskError(FindTaskError)
}

impl From<FindTaskError> for HandleCommandError {
  fn from(other: FindTaskError) -> Self {
    HandleCommandError::FindTaskError(other)
  }
}

fn handle_command(model: &mut Model, command: Command) -> Result<Option<Effect>, HandleCommandError> {
  info!("Command: {:?}", command);

  match command {
    Command::List => {
      model.recalculate_numerical_ids();

      // TODO: Calculate padding
      let right_padding = 10 + 8;
      let terminal_width = terminal_size().columns - right_padding;


      // TODO: Find a nicer way to pass a slice of slices of
      // string-slices
      let rows: Vec<_> = model.all_tasks().into_iter()
        .filter(|t| !t.is_done())
        .map(|task| {
          let short = model.numerical_ids.get(&task.uuid)
            .map(u64::to_string)
            .unwrap_or(task.short_id());

          vec![short,
               task.priority.to_string(),
               task.age().to_string(),
               task.description.clone(),
               format!("{:.2}", task.urgency())]
        }).collect();

      use std::io;
      let mut p = TablePrinter::new();
      p.rows = rows;
      p.width_limit = Some(terminal_width);
      p.add_column("id");
      p.add_column("pri");
      p.add_column("age");
      p.add_column("desc");
      p.add_column("urg");
      p.calculate_widths();
      p.print(&mut io::stdout()).unwrap();

      Ok(None)
    },
    Command::Show(r) => {
      let task = try!(model.find_task(&r));
      println!("{:?}", task);
      Ok(None)
    },
    Command::Add(title, tags) => {
      Ok(Some(Effect::AddTask(Task::new_with_tags(&title, tags))))
    },
    Command::Delete(r) => {
      let task = try!(model.find_task(&r));
      Ok(Some(Effect::DeleteTask(task.uuid.clone())))
    },
    Command::MarkDone(r) => {
      let task = try!(model.find_task(&r));
      Ok(Some(Effect::ChangeTaskState(task.uuid.clone(), TaskState::Done(time::get_time()))))
    },
    Command::ChangePriority(r, p) => {
      let task = try!(model.find_task(&r));
      Ok(Some(Effect::ChangeTaskPriority(task.uuid.clone(), p)))
    },
  }
}

fn main() {
  env_logger::init().unwrap();

  chdir();
  let _lock = FileLock::new(PID_FILE)
    .expect("Failed to acquire lock");

  let mut store = Storage::new().expect("Failed to open store");

  let command = Command::from_args();

  if let Err(e) = command {
    println!("Error while parsing command: {}", e.0);
    return;
  } else if let Ok(command) = command {
    let mut model = store.model();
    let effect = handle_command(&mut model, command).unwrap(); // TODO

    // Print effect descriptions
    if let Some(ref effect) = effect {
      use rtask::Effect::*;
      match *effect {
        AddTask(ref t)       => println!("Added Task '{}'", t.description),
        DeleteTask(ref uuid) => println!("Deleted task '{}'", model.tasks[uuid].description),
        ChangeTaskState(ref uuid, ref state) => {
          let ref t = model.tasks[uuid];
          match *state {
            TaskState::Done(_) => println!("Marking task '{}' as done", t.description),
            TaskState::Open    => println!("Marking task '{}' as open", t.description),
          }
        },
        ChangeTaskPriority(ref uuid, ref priority) => {
          let ref t = model.tasks[uuid];
          println!("Changing  priority of task '{}' to {}", t.description, priority);
        }
      }
    }

    
    effect.map(|e| model.apply_effect(e));
  } else {
    panic!("Invalid command :-(")
  }

  mem::drop(store);
  fs::remove_file(PID_FILE).expect("Failed to remove pid file");
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
