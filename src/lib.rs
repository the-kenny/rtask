extern crate ansi_term;
extern crate bincode;
extern crate libc;
extern crate regex;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate time;
extern crate uuid;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;

// Todoist Integration
#[cfg(feature = "todoist")] extern crate hyper;
#[cfg(feature = "todoist")] pub mod todoist;

pub mod commands;
pub mod file_lock;
pub mod model;
pub mod printer;
pub mod task;
pub mod task_ref;
pub mod terminal_size;

pub use commands::*;
pub use task::*;
pub use model::*;
pub use task_ref::*;
pub use file_lock::FileLock;

pub trait StorageEngine: Sized + Drop {
  type LoadErr;
  // fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, Self::LoadErr>;
  fn new() -> Result<Self, Self::LoadErr>;
  fn model<'a>(&'a mut self) -> &'a mut Model;
}

mod sqlite_storage;
pub type Storage = sqlite_storage::SqliteStorage;
