extern crate chrono;
extern crate rtask;
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
extern crate env_logger;

use rtask::command::{Command, Flag};
use rtask::terminal_size::*;
use rtask::*;

use std::io::ErrorKind;
use std::{env, fmt, fs, io, mem};

pub const PID_FILE: &'static str = "tasks.pid";

#[derive(Debug, PartialEq, Eq)]
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

// TODO: Use a better error enum
#[derive(Debug, Fail)]
enum HandleCommandError {
    #[fail(display = "Failed to find task: {}", _0)]
    FindTaskError(FindTaskError),
}

impl From<FindTaskError> for HandleCommandError {
    fn from(other: FindTaskError) -> Self {
        HandleCommandError::FindTaskError(other)
    }
}

// TODO: move to rtask crate
fn command_to_effects(
    model: &mut Model,
    command: Command,
) -> Result<Vec<Effect>, HandleCommandError> {
    info!("Command (prior scope handling): {:?}", command);
    let scope = Scope(env::var("RTASK_SCOPE").ok());

    info!("Using scope {:?}", scope);

    match command {
        Command::List(mut flags) => {
            use rtask::printer::*;

            scope.as_tag().map(|t| flags.push(Flag::TagPositive(t)));

            info!("Listing filtered by flags {:?}", flags);

            if !flags.is_empty() {
                let flags = flags
                    .iter()
                    .map(|f| format!("{}", f))
                    .collect::<Vec<String>>()
                    .join(", ");
                println!("Listing all tasks with flags {}", flags);
            }

            let task_ids: Vec<_> = model
                .all_tasks()
                .into_iter()
                .filter(|t| t.is_open())
                .filter(|t| flags.is_empty() || flags.iter().all(|f| f.matches(&t)))
                .map(|t| t.uuid)
                .collect();

            // Recalculate IDs
            model.recalculate_numerical_ids(&scope, &task_ids[..]);

            let terminal_size = terminal_size();

            let filtered_tasks: Vec<_> = task_ids
                .iter()
                .map(|uuid| model.tasks.get(uuid).unwrap())
                .collect();

            let task_limit = terminal_size.rows - 4; // TODO: Use a better number

            let rows: Vec<_> = filtered_tasks
                .iter()
                .enumerate()
                .map(|(n, task)| {
                    let short = model
                        .short_task_id(&scope, &task.uuid)
                        .map(|n| n.to_string())
                        .unwrap_or(task.short_id());

                    let values = vec![
                        short,
                        task.priority.to_string(),
                        task.age().to_string(),
                        task.description.clone(),
                        format!("{:.2}", task.urgency()),
                    ];

                    let mut style = Style::default();
                    if n % 2 == 0 {
                        style = style.on(Colour::RGB(40, 40, 40))
                    };
                    style = match task.priority {
                        Priority::High => style.fg(Colour::RGB(250, 50, 50)),
                        Priority::Low => style.fg(Colour::RGB(150, 150, 150)),
                        _ => style,
                    };

                    PrintRow {
                        fields: values,
                        style: Some(style),
                    }
                })
                .take(task_limit)
                .collect();

            if !rows.is_empty() {
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

            Ok(vec![])
        }
        Command::Show(refs) => {
            for task_ref in refs {
                let task = model.find_task(&scope, &task_ref);

                if task.is_err() {
                    println!("Couldn't find task {}", task_ref);
                    continue;
                }

                let task = task.unwrap();

                match scope.as_tag() {
                    Some(ref t) if !task.tags.contains(t) => {
                        println!("Note: Task {} isn't in scope {}", task_ref, scope);
                    }
                    _ => (),
                }

                macro_rules! p {
          ( $( ($k:ident, $v:expr), )* ) => {
            $(
              println!("{:<15} {}", stringify!($k), $v);
            )*
          }
        }

                let tag_list = task.tags
                    .iter()
                    .map(|s| &s[..])
                    .collect::<Vec<_>>()
                    .join(", ");

                println!("==== task {} ====", task_ref);

                p!(
                    (uuid, task.uuid),
                    (description, task.description),
                    (priority, task.priority),
                    (created, task.created),
                    (modified, task.modified),
                    (tags, tag_list),
                    (extras, format!("{:?}", task.extras)),
                );
            }

            Ok(vec![])
        }
        Command::Add(title, flags) => {
            // If in a scope, add scope-tag to `tags`
            let tags = if let Some(tag) = scope.as_tag() {
                vec![tag]
            } else {
                vec![]
            };

            info!("Got flags: {:?}", flags);

            let mut task = Task::new_with_tags(&title, tags.into_iter().collect());
            for flag in flags {
                flag.apply_to(&mut task);
            }

            Ok(vec![Effect::AddTask(task)])
        }
        Command::Delete(refs) => {
            let effects = refs.iter()
                .flat_map(|tr| model.find_task(&scope, tr))
                .map(|t| Effect::DeleteTask(t.uuid.clone()))
                .collect();

            Ok(effects)
        }
        Command::MarkDone(refs) => {
            let state = TaskState::Done(chrono::Utc::now());
            let effects = refs.iter()
                .flat_map(|tr| model.find_task(&scope, tr))
                .map(|t| Effect::ChangeTaskState(t.uuid.clone(), state.clone()))
                .collect();

            Ok(effects)
        }
        Command::MarkCanceled(refs) => {
            //
            let state = TaskState::Canceled(chrono::Utc::now());
            let effects = refs.iter()
                .flat_map(|tr| model.find_task(&scope, tr))
                .map(|t| Effect::ChangeTaskState(t.uuid.clone(), state.clone()))
                .collect();

            Ok(effects)
        }
        Command::ChangeTaskProperties {
            task_refs,
            added_tags,
            removed_tags,
            priority,
        } => {
            let mut effects = vec![];

            for task_ref in task_refs {
                let task = model.find_task(&scope, &task_ref)?;

                if let Some(p) = priority {
                    effects.push(Effect::ChangeTaskPriority(task.uuid.clone(), p));
                }

                if !added_tags.is_empty() || !removed_tags.is_empty() {
                    effects.push(Effect::ChangeTaskTags {
                        uuid: task.uuid.clone(),
                        added: added_tags.clone(),
                        removed: removed_tags.clone(),
                    });
                }
            }

            Ok(effects)
        }
    }
}

fn main() {
    env_logger::init();
    chdir();

    let mut lock = FileLock::new(PID_FILE).expect("Failed to acquire lock");
    lock.delete_on_drop = true;

    let mut store = Storage::new().expect("Failed to open store");
    let command = Command::from_args();

    match command {
        Err(error) => {
            println!("Error while parsing command: {}", error.0);
            return;
        }
        Ok(command) => {
            let mut model = store.model();
            match command_to_effects(&mut model, command) {
                // TODO: Store TaskRef in these errors (and simply the naming)
                Err(HandleCommandError::FindTaskError(FindTaskError::MultipleResults)) => {
                    println!("Multiple matching tasks found");
                }
                Err(HandleCommandError::FindTaskError(FindTaskError::TaskNotFound)) => {
                    println!("No matching task found");
                }
                Ok(effects) => {
                    for effect in effects {
                        info!("Applying Effect: {:?}", effect);

                        model.apply_effect(&effect);
                        effect.print(&model, &mut io::stdout()).unwrap();
                    }
                }
            }
        }
    }

    mem::drop(store);
}

fn chdir() {
    let dir = env::var("RTASK_DIRECTORY")
        .map(Into::into)
        .unwrap_or_else(|_| {
            let mut dir = env::home_dir().expect("Couldn't get home dir");
            dir.push(".rtask/");
            dir
        });
    fs::create_dir_all(&dir).expect("Failed to create directory");

    let dir = dir.canonicalize().expect("Failed to get absolute path");

    info!("Working directory: {}", dir.display());

    match fs::create_dir(&dir) {
        Err(ref err) if err.kind() == ErrorKind::AlreadyExists => (),
        Ok(_) => (),
        Err(effect) => panic!("Couldn't create {}: {}", &dir.display(), effect),
    }

    env::set_current_dir(&dir).expect("Couldn't chdir");
}

#[cfg(test)]
mod tests {
    use super::rtask::command::*;
    use super::rtask::*;

    #[test]
    fn test_command_to_effect_no_noop_effects() {
        let mut m = Model::new();
        let t = Task::new("bar");
        m.apply_effect(&Effect::AddTask(t.clone()));

        let c = Command::ChangeTaskProperties {
            task_refs: vec![t.uuid.into()],
            added_tags: Tags::new(),
            removed_tags: Tags::new(),
            priority: None,
        };

        let effects = super::command_to_effects(&mut m, c).unwrap();
        assert!(effects.is_empty());
    }
}
