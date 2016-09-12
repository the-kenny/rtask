extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate time;

use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::{env, fs};
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

fn command_to_effect(model: &mut Model,
                     scope: &Scope,
                     command: Command)
                     -> Result<Option<Effect>, HandleCommandError> {
  info!("Command (prior scope application): {:?}", command);

  match command {
    Command::List(tags) => {
      use rtask::printer::*;

      info!("Listing filtered by tags {:?}", tags);

      if !tags.is_empty() {
        println!("Listing all tasks with tag(s) {:?}", tags);
      }

      let task_ids: Vec<_> = model.all_tasks().into_iter()
        .filter(|t| !t.is_done())
        .filter(|t| tags.is_empty() || tags.is_subset(&t.tags))
        .filter(|t| scope.contains_task(t))
        .map(|t| t.uuid)
        .collect();

      // Recalculate IDs
      // TODO: Scope Handling
      model.recalculate_numerical_ids(&task_ids[..]);

      let terminal_size = terminal_size();

      let filtered_tasks: Vec<_> = task_ids.iter()
        .map(|uuid| model.tasks.get(uuid).unwrap())
        .collect();

      let task_limit = terminal_size.rows - 4; // TODO: Use a better number

      let rows: Vec<_> = filtered_tasks.iter()
        .enumerate()
        .map(|(n, task)| {
          let short = model.short_task_id(&task.uuid)
            .map(|n| n.to_string())
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
        }).take(task_limit).collect();

      if !rows.is_empty() {
        use std::io;
        let mut p = TablePrinter::new();
        p.width_limit = Some(terminal_size.columns - 12);
        p.titles = vec!["id", "pri", "age", "desc", "urg"];
        p.print(&mut io::stdout(), &rows).unwrap();

        if filtered_tasks.len() > rows.len() {
          println!("There are {} more tasks", filtered_tasks.len() - rows.len());
        }
      } else {
        println!("No matching tasks found");
      }

      Ok(None)
    },
    Command::Show(r) => {
      let task = try!(model.find_task(&r));
      if !scope.contains_task(&task) {
        println!("Note: Task {} isn't in scope", r);
      }

      println!("{:?}", task);
      Ok(None)
    },
    Command::Add(title, mut tags) => {
      tags.extend(scope.default_tags.iter().cloned());
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
    Command::ChangeTags{ task_ref, added, removed } => {
      let task = try!(model.find_task(&task_ref));
      // TODO: Warn when scope-tags are removed
      Ok(Some(Effect::ChangeTaskTags{
        uuid: task.uuid.clone(),
        added: added,
        removed: removed,
      }))
    }
  }
}

fn main() {
  env_logger::init().unwrap();
  chdir();

  let _lock = FileLock::new(PID_FILE).expect("Failed to acquire lock");

  {
    // TODO: Load from file
    let mut bevuta = Scope::default();
    bevuta.included_tags.insert("bevuta".into());
    bevuta.default_tags.insert("bevuta".into());
    let mut config = Config::default();
    config.scopes.insert("bevuta".into(), bevuta);

    let scope = env::var("RTASK_SCOPE").ok()
      .and_then(|v| config.scopes.get(&v))
      .unwrap_or(&config.default_scope);

    info!("Using scope: {:?}", scope);

    let mut store = Storage::new().expect("Failed to open store");
    let command = Command::from_args();

    match command {
      Err(effect) => {
        println!("Error while parsing command: {}", effect.0);
        return;
      },
      Ok(command) => {
        let mut model = store.model();
        let effect = command_to_effect(&mut model, &scope, command).unwrap(); // TODO

        // Print effect descriptions
        if let Some(ref effect) = effect {
          use rtask::Effect::*;
          match *effect {
            AddTask(ref t)       => println!("Added Task '{}'", t.description),
            DeleteTask(ref uuid) => println!("Deleted task '{}'", model.tasks[uuid].description),
            ChangeTaskTags{ ref added, ref removed, ..} => {
              if !added.is_empty()   { println!("Added tags {:?}",   added); }
              if !removed.is_empty() { println!("Removed tags {:?}", removed); }
            }
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

        effect.map(|effect| {
          info!("Effect: {:?}", effect);
          model.apply_effect(effect)
        });
      },
    }
  }

  fs::remove_file(PID_FILE).expect("Failed to remove pid file");
}

fn chdir() {
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
    Err(effect) => panic!("Couldn't create {}: {}", &dir.display(), effect),
  }

  env::set_current_dir(&dir).expect("Couldn't chdir");
}
