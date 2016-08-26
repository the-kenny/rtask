extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate time;

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
        model.recalculate_numerical_ids();

        // TODO: Calculate padding
        let right_padding = 10 + 8;
        let terminal_width = terminal_size().columns - right_padding;

        use std::io;

        // TODO: Find a nicer way to pass a slice of slices of
        // string-slices
        let rows: Vec<_> = model.all_tasks().into_iter()
          .filter(|t| !t.is_done())
          .map(|task| {
            let short = model.numerical_ids.get(&task.uuid)
              .map(u64::to_string)
              .unwrap_or(task.short_id());

            vec![short,
                 task.age().to_string(),
                 task.description.clone(),
                 task.urgency().to_string()]
          }).collect();

        let mut p = TablePrinter::new();
        p.add_column("id");
        p.add_column("age");
        p.add_column("desc");
        p.add_column("urg");
        p.rows = rows;
        p.calculate_widths();
        p.print(&mut io::stdout()).unwrap();

        None
      },
      Command::Show(r) => {
        match model.find_task(&r) {
          Ok(task)             => println!("{:?}", task),
          Err(TaskNotFound)    => println!("Found no tasks matching {}", r),
          Err(MultipleResults) => println!("Found multiple tasks matching {}", r),
        }

        None
      },
      Command::Add(title, tags) => {
        Some(Effect::AddTask(Task::new_with_tags(&title, tags)))
      },
      Command::Delete(r) => {
        match model.find_task(&r) {
          Ok(task) => {
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
      },
      Command::MarkDone(r) => {
        match model.find_task(&r) {
          Ok(task) => {
            Some(Effect::ChangeTaskState(task.uuid.clone(), TaskState::Done(time::get_time())))
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

    // Print effect descriptions
    if let Some(ref effect) = effect {
      match *effect {
        Effect::AddTask(ref t)       => println!("Added Task '{}'", t.description),
        Effect::DeleteTask(ref uuid) => println!("Deleted task '{}'", model.tasks[uuid].description),
        Effect::ChangeTaskState(ref uuid, ref state) => {
          let ref t = model.tasks[uuid];
          match *state {
            TaskState::Done(_) => println!("Marking task '{}' as done", t.description),
            TaskState::Open    => println!("Marking task '{}' as open", t.description),
          }
        },
      }
    }

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
