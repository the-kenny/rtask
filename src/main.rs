extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate time;

use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::fs;
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

fn command_to_effect(model: &Model, command: Command) -> Result<Option<Effect>, HandleCommandError> {
  info!("Command: {:?}", command);

  match command {
    Command::List(ref tags) => {
      use rtask::printer::*;

      info!("Listing filtered by tags {:?}", tags);

      if !tags.is_empty() {
        println!("Listing all tasks with tag(s) {:?}", tags);
      }

      // TODO: Calculate padding
      let right_padding = 10 + 8;
      let terminal_width = terminal_size().columns - right_padding;
      let rows: Vec<_> = model.all_tasks().into_iter()
        .filter(|t| !t.is_done())
        .filter(|t| tags.is_empty() || tags.is_subset(&t.tags))
        .enumerate()
        .map(|(n, task)| {
          let short = model.numerical_ids.get(&task.uuid)
            .map(u64::to_string)
            .unwrap_or(task.short_id());

          let values = vec![short,
                            task.priority.to_string(),
                            task.age().to_string(),
                            task.description.clone(),
                            format!("{:.2}", task.urgency())];

          let mut style = Style::default();
          if n % 2 == 0 { style = style.on(Colour::RGB(40,40,40)) };
          style = match task.priority {
            Priority::High => style.fg(Colour::RGB(250,50,50)),
            Priority::Low  => style.fg(Colour::RGB(150,150,150)),
            _ => style
          };

          PrintRow {
            fields: values,
            style: Some(style),
          }
        }).collect();

      if !rows.is_empty() {
        use std::io;
        let mut p = TablePrinter::new();
        p.width_limit = Some(terminal_width);
        p.titles = vec!["id", "pri", "age", "desc", "urg"];
        p.print(&mut io::stdout(), &rows).unwrap();
      } else {
        println!("No matching tasks found");
      }

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

  let _lock = FileLock::new(PID_FILE).expect("Failed to acquire lock");

  {
    let mut store = Storage::new().expect("Failed to open store");
    let command = Command::from_args();

    match command {
      Err(e) => {
        println!("Error while parsing command: {}", e.0);
        return;
      },
      Ok(command) => {
        let mut model = store.model();

        if command.should_recalculate_ids() {
          model.recalculate_numerical_ids();
        }

        let effect = command_to_effect(&mut model, command).unwrap(); // TODO

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
      },
    }
  }

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
