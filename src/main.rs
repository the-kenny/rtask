extern crate rtask;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate time;

use rtask::*;

use rtask::commands::Command;
use rtask::terminal_size::*;

use std::{env, fmt, fs, mem};
use std::io::ErrorKind;

pub const PID_FILE: &'static str = "tasks.pid";

#[derive(Debug,PartialEq,Eq)]
struct Scope(Option<String>);

use std::ops::Deref;
impl Deref for Scope {
  type Target = str;
  fn deref(&self) -> &str {
    // .as_ref().map(AsRef::as_ref), wtf?!
    self.0.as_ref().map(AsRef::as_ref).unwrap_or("default")
  }
}

impl fmt::Display for Scope {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_str(self.as_ref())
  }
}

impl Scope {
  pub fn as_tag(&self) -> Option<Tag> {
    self.0.clone()
  }
}

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
                     command: Command)
                     -> Result<Option<Effect>, HandleCommandError> {

  info!("Command (prior scope handling): {:?}", command);
  let scope = Scope(env::var("RTASK_SCOPE").ok());

  info!("Using scope {}", scope);

  match command {
    Command::List(mut tags) => {
      use rtask::printer::*;

      scope.as_tag().map(|t| tags.insert(t));

      info!("Listing filtered by tags {:?}", tags);

      if !tags.is_empty() {
        println!("Listing all tasks with tag(s) {:?}", tags);
      }

      let task_ids: Vec<_> = model.all_tasks().into_iter()
        .filter(|t| t.is_open())
        .filter(|t| tags.is_empty() || tags.is_subset(&t.tags))
        .map(|t| t.uuid)
        .collect();

      // Recalculate IDs
      model.recalculate_numerical_ids(&scope, &task_ids[..]);

      let terminal_size = terminal_size();

      let filtered_tasks: Vec<_> = task_ids.iter()
        .map(|uuid| model.tasks.get(uuid).unwrap())
        .collect();

      let task_limit = terminal_size.rows - 4; // TODO: Use a better number

      let rows: Vec<_> = filtered_tasks.iter()
        .enumerate()
        .map(|(n, task)| {
          let short = model.short_task_id(&scope, &task.uuid)
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
        p.alignments.insert("desc", Alignment::Left);
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
      let task = try!(model.find_task(&scope, &r));
      match scope.as_tag() {
        Some(ref t) if !task.tags.contains(t) => {
          println!("Note: Task {} isn't in scope {}", r, scope);
        }
        _ => ()
      }

      println!("{:?}", task);
      Ok(None)
    },
    Command::Add(title, mut tags) => {
      scope.as_tag().map(|t| tags.insert(t));
      Ok(Some(Effect::AddTask(Task::new_with_tags(&title, tags))))
    },
    Command::Delete(r) => {
      let task = try!(model.find_task(&scope, &r));
      Ok(Some(Effect::DeleteTask(task.uuid.clone())))
    },
    Command::MarkDone(r) => {
      let task = try!(model.find_task(&scope, &r));
      Ok(Some(Effect::ChangeTaskState(task.uuid.clone(), TaskState::Done(time::get_time()))))
    },
    Command::MarkCanceled(r) => {
      let task = try!(model.find_task(&scope, &r));
      Ok(Some(Effect::ChangeTaskState(task.uuid.clone(), TaskState::Canceled(time::get_time()))))
    },
    Command::ChangePriority(r, p) => {
      let task = try!(model.find_task(&scope, &r));
      Ok(Some(Effect::ChangeTaskPriority(task.uuid.clone(), p)))
    },
    Command::ChangeTags{ task_ref, added, removed } => {
      let task = try!(model.find_task(&scope, &task_ref));
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

  let mut store = Storage::new().expect("Failed to open store");
  let command = Command::from_args();

  match command {
    Err(error) => {
      println!("Error while parsing command: {}", error.0);
      return;
    },
    Ok(command) => {
      let mut model = store.model();
      match  command_to_effect(&mut model, command) {
        // TODO: Store TaskRef in these errors (and simply the naming)
        Err(HandleCommandError::FindTaskError(FindTaskError::MultipleResults)) => {
          println!("Multiple matching tasks found");
        },
        Err(HandleCommandError::FindTaskError(FindTaskError::TaskNotFound)) => {
          println!("No matching task found");
        },
        Ok(None) => (),
        Ok(Some(effect)) => {
          // Ugh, cloning (to make the borrow checker happy)
          let t = effect.task_id().and_then(|id| model.get_task(id)).cloned();

          info!("Effect: {:?}", effect);
          model.apply_effect(effect.clone());

          // Print effect descriptions
          use rtask::Effect::*;
          match effect {
            AddTask(ref t)       => println!("Added Task {}", t.short_id()),
            DeleteTask(_)        => println!("Deleted task '{}'", t.unwrap().description),
            ChangeTaskTags{ ref added, ref removed, ..} => {
              if !added.is_empty()   { println!("Added tags {:?}",   added); }
              if !removed.is_empty() { println!("Removed tags {:?}", removed); }
            }
            ChangeTaskState(_uuid, ref state) => {
              match *state {
                TaskState::Done(_)     => println!("Marking task '{}' as done", t.unwrap().description),
                TaskState::Open        => println!("Marking task '{}' as open", t.unwrap().description),
                TaskState::Canceled(_) => println!("Marking task '{}' as canceled", t.unwrap().description),
              }
            },
            ChangeTaskPriority(_uuid, ref priority) => {
              println!("Changed priority of task '{}' to {}", t.unwrap().description, priority);
            }
          }
        }
      }
    },
  }
  
  mem::drop(store);
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
