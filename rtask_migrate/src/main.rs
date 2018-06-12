extern crate rusqlite;
extern crate failure;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rustc_serialize;
extern crate uuid;
extern crate chrono;
extern crate time;

use std::env;
use std::collections::{HashMap, HashSet};

pub type Title    = String;
pub type Time     = chrono::DateTime<chrono::Utc>;
pub type Uuid     = String; // Hack
pub type Tag      = String;
pub type Tags     = HashSet<Tag>;
pub type ExtraMap = HashMap<ExtraData, String>;

pub struct Age(chrono::Duration);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TaskState {
  Open,
  Done(Time),
  Canceled(Time),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, RustcDecodable)]
enum TaskStateOld {
  Open,
  Done(time::Timespec),
  Canceled(time::Timespec),
}

impl rustc_serialize::Decodable for TaskState {
  fn decode<D: rustc_serialize::Decoder>(d: &mut D) -> Result<TaskState, D::Error> {
    let value: TaskStateOld = rustc_serialize::Decodable::decode(d)?;
    let utc = chrono::Utc;
    let x = match value {
      TaskStateOld::Open => TaskState::Open,
      TaskStateOld::Done(t) => TaskState::Done(Time::from_utc(chrono::NaiveDateTime::from_timestamp(t.sec, t.nsec as u32), utc)),
      TaskStateOld::Canceled(t) => TaskState::Canceled(Time::from_utc(chrono::NaiveDateTime::from_timestamp(t.sec, t.nsec as u32), utc)),
      
    };
    Ok(x)
  }
}



#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, RustcDecodable)]
pub enum Priority {
  Low,
  Default,
  High,
  // Custom(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, RustcDecodable)]
pub enum ExtraData {
  Notes = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Task {
  pub description: Title,
  pub status:      TaskState,
  pub priority:    Priority,
  pub created:     Time,
  pub modified:    Time,
  pub uuid:        Uuid,
  pub tags:        Tags,
  pub extras:      ExtraMap,
}

use rustc_serialize::{Decoder,Decodable};
impl Decodable for Task {
  fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
    let description = try!(d.read_struct_field("description", 0, Decodable::decode));
    let status      = try!(d.read_struct_field("status",      0, Decodable::decode));
    // Hack to allow missing fields (for backwards compatibility)
    let priority: Option<Priority> = try!(d.read_struct_field("priority",    0, Decodable::decode));
    
    let created: time::Timespec  = try!(d.read_struct_field("created",     0, Decodable::decode));
    let modified: time::Timespec = try!(d.read_struct_field("modified",    0, Decodable::decode));
    
    let uuid        = try!(d.read_struct_field("uuid",        0, Decodable::decode));
    let tags        = try!(d.read_struct_field("tags",        0, Decodable::decode));
    let extras      = try!(d.read_struct_field("extras",      0, Decodable::decode));

    let utc = chrono::Utc;
    Ok(Task {
      description: description,
      status: status,
      priority: priority.unwrap_or(Priority::Default),
      created: Time::from_utc(chrono::NaiveDateTime::from_timestamp(created.sec, created.nsec as u32), utc),
      modified: Time::from_utc(chrono::NaiveDateTime::from_timestamp(modified.sec, modified.nsec as u32), utc),
      uuid: uuid,
      tags: tags,
      extras: extras,
    })
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize , RustcDecodable)]
pub enum Effect {
  AddTask(Task),
  ChangeTaskTags { uuid: Uuid, added: Tags, removed: Tags },
  ChangeTaskState(Uuid, TaskState),
  ChangeTaskPriority(Uuid, Priority),
  DeleteTask(Uuid),
  // Undo,
}


fn main() -> Result<(), failure::Error> {
  if let Some(db_path) = env::args().skip(1).next() {
    let mut con = rusqlite::Connection::open(&db_path)?;
    let mut tx = con.transaction()?;

    {
      tx.execute("drop trigger no_upate_trigger;", &[])?;
      tx.execute("drop trigger no_delete_trigger;", &[])?;
      
      let mut stmt = tx.prepare("select id, json from effects order by id")?;
      let rows = stmt.query_map(&[], |row| {
        let id: i64 = row.get(0);
        let json_str: String = row.get(1);

        (id, json_str)
      })?.map(|x| x.unwrap())
        .map(|(id, json_str)| {
          let effect: Effect = rustc_serialize::json::decode(&json_str).unwrap();
          // let effect: serde_json::Value = serde_json::from_str(&json_str).unwrap();
          (id, effect) 
        })
        .collect::<Vec<_>>();

      let mut update_stmt = tx.prepare("UPDATE effects SET json = ? where id = ?")?;
      for (id, effect) in rows {
        let new_json = serde_json::to_string(&effect).unwrap();
        println!("new_json: {:?}", new_json);
        update_stmt.execute(&[&new_json, &id])?;
      }
    }

    tx.commit()?;
    
    Ok(())
  } else {
    Err(failure::err_msg("Invalid Args"))
  }
}
