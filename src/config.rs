use std::collections::{HashMap};

use ::task::{Task,Tags};

#[derive(Debug, PartialEq, Eq)]
pub struct Scope {
  pub excluded_tags: Tags,
  pub included_tags: Tags,
  // TODO: `default_tags`
}

impl Scope {
  pub fn contains_task(&self, t: &Task) -> bool {
    self.excluded_tags.is_disjoint(&t.tags) && self.included_tags.is_subset(&t.tags)

  }
}

impl Default for Scope {
  fn default() -> Self {
    Scope {
      excluded_tags: Tags::new(),
      included_tags: Tags::new(),
    }
  }
}

pub struct Config {
  pub scopes: HashMap<String, Scope>,
  pub default_scope: Scope,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      scopes: HashMap::new(),
      default_scope: Scope::default(),
    }
  }
}
