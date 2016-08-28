use super::{ Model };

pub trait StorageEngine: Sized + Drop {
  type LoadErr;
  // fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, Self::LoadErr>;
  fn new() -> Result<Self, Self::LoadErr>;
  fn model<'a>(&'a mut self) -> &'a mut Model;
}

