use super::{Effect, Model, Task, TaskState, Tags};

use std::collections::BTreeMap;

use hyper;
use hyper::Client;
use hyper::header::ContentType;
use rustc_serialize::json;

pub type ApiToken = String;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
  FetchData,
}

quick_error! {
  #[derive(Debug)]
  pub enum Error {
    Json(err: json::ParserError) {
      from()
    }
    Http(err: hyper::Error) {
      from()
    }
  }
}

const CUSTOM_DATA_PROJECT_PREFIX: &'static str = "todoist_project_";
const CUSTOM_DATA_SYNC_TOKEN: &'static str = "todoist_sync_token";

type Id = u64;


struct TodoistTask(Task);
struct TodoistTaskState(TaskState);

impl<'a> From<&'a json::Json> for TodoistTaskState {
  fn from(json: &'a json::Json) -> Self {
    use rustc_serialize::json::Json::*;

    let s = match (&json["checked"], &json["is_deleted"], &json["is_archived"]) {
      (&Boolean(true), _, _) => TaskState::done(),
      (_, &Boolean(true), _) => TaskState::canceled(),
      (_, _, &Boolean(true)) => TaskState::canceled(),
      _ => TaskState::Open,
    };

    TodoistTaskState(s)
  }
}

impl<'a> From<&'a json::Json> for TodoistTask {
  fn from(json: &'a json::Json) -> Self {
    let title = json["content"].as_string().unwrap();

    // status: TaskState::Open,
    // priority: Priority::default(),
    // created: now,
    // modified: now,
    // uuid: Uuid::new_v4(),
    // tags: Tags::new(),
    // extras: ExtraMap::new(),

    TodoistTask(Task {
      description: title.to_string(),
      status: TodoistTaskState::from(json).0,
      ..Default::default()
    })
  }
}

pub fn command_to_effect(model: &Model, command: Command) -> Result<Vec<Effect>, Error> {
  let mut effects: Vec<Effect> = vec![];

  match command {
    Command::FetchData => {
      let sync_token = model.custom_data.get(CUSTOM_DATA_SYNC_TOKEN)
        .map(|x| x.as_ref())
        .unwrap_or("*");

      let client = Client::new();
      let mut res = client.post("https://todoist.com/API/v7/sync")
        .header(ContentType("application/x-www-form-urlencoded".parse().unwrap()))
        .body(&format!("token=abe6513d04f5ce31f32ed810cc0e8eac593297bd&sync_token={sync_token}&resource_types={resources}",
                       sync_token=sync_token,
                       resources="[\"items\", \"projects\"]"))
        .send()?;

      let json = json::Json::from_reader(&mut res)?;

      let new_sync_token = json["sync_token"].as_string().unwrap();
      if json["full_sync"].as_boolean().unwrap() {
        warn!("Doing full sync");
      }

      if new_sync_token != sync_token {
        effects.push(Effect::CustomData(CUSTOM_DATA_SYNC_TOKEN.to_string(),
                                        new_sync_token.to_string()));
      }

      let persisted_projects: BTreeMap<Id, String> = model.custom_data.iter()
        .filter_map(|(ref k, ref v)| {
          if k.starts_with(CUSTOM_DATA_PROJECT_PREFIX) {
            if let Ok(id) = Id::from_str_radix(k.trim_left_matches(CUSTOM_DATA_PROJECT_PREFIX), 10) {
              Some((id, v.to_string()))
            } else {
              panic!("Couldn't parse project-id {}", k);
            }
          } else {
            None
          }
        }).collect();

      info!("Found {} existing projects", persisted_projects.len());

      let new_projects: BTreeMap<_, _> = json["projects"]
        .as_array().unwrap()
        .into_iter()
        .filter_map(|json| {
          let id = json["id"].as_u64().unwrap();
          if !persisted_projects.contains_key(&id) {
            Some((id, json["name"].as_string().unwrap().to_string()))
          } else {
            None
          }
        }).collect();

      info!("Found {} new projects", new_projects.len());

      // Emit Effect::CustomData for each new project
      effects.extend(new_projects.iter().map(|(id, name)| {
        Effect::CustomData(format!("{}{}", CUSTOM_DATA_PROJECT_PREFIX, id), name.clone())
      }));

      let all_projects = {
        let mut p = persisted_projects;
        p.extend(new_projects.into_iter());
        p
      };

      if let Some(items) = json["items"].as_array() {
        info!("Got {} items from Todoist", items.len());

        let new_tasks = items.into_iter()
          .filter_map(|json| {
            let mut tags = Tags::default();

            if let Some(project) = json["project_id"].as_u64().and_then(|id| all_projects.get(&id)) {
              tags.insert(project.to_string().to_lowercase());
            }

            let t: TodoistTask = json.into();
            let mut t = t.0;
            t.tags = tags;

            Some(t)
          })
          .map(Effect::AddTask);

        effects.extend(new_tasks);

        // for item in items {
        //   println!("{}", item.pretty());
        // }
      }

      println!("{:?}", sync_token);
    }
  }

  Ok(effects)
}
