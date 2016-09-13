use std::collections::{HashMap};
use std::path::Path;
use std::fs::File;
use std::io;
use toml;

use ::task::{Task,Tags};

#[derive(Debug, PartialEq, Eq)]
pub struct Scope {
  pub name: String,
  pub excluded_tags: Tags,
  pub included_tags: Tags,
  pub default_tags: Tags,
}

impl Scope {
  pub fn contains_task(&self, t: &Task) -> bool {
    self.excluded_tags.is_disjoint(&t.tags) && self.included_tags.is_subset(&t.tags)
  }
}

impl Default for Scope {
  fn default() -> Self {
    Scope {
      name: "default".into(),
      excluded_tags: Tags::new(),
      included_tags: Tags::new(),
      default_tags: Tags::new(),
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

impl Config {
  pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
    use std::io::Read;

    let mut data = String::new();
    let mut file = try!(File::open(path));
    try!(file.read_to_string(&mut data));
    let parsed = toml::Parser::new(&data).parse().unwrap();

    println!("parsed: {:?}", parsed);

    let mut config = Config::default();
    if let Some(scopes) = parsed.get("scopes").and_then(toml::Value::as_table) {
      for (scope_name, values) in scopes {
        let scope: Option<Scope> = toml::decode(values.clone());
        println!("{:?}", scope);
        // let default_tags = values.lookup("default_tags")
        //   .cloned()
        //   .and_then(toml::decode)
        //   .unwrap_or(Tags::new());
        // let included_tags = values.lookup("included_tags")
        //   .cloned()
        //   .and_then(toml::decode)
        //   .unwrap_or(Tags::new());
        // let excluded_tags = values.lookup("excluded_tags")
        //   .cloned()
        //   .and_then(toml::decode)
        //   .unwrap_or(Tags::new());

        // let scope = Scope {
        //   name: scope_name.clone(),
        //   default_tags: default_tags,
        //   excluded_tags: excluded_tags,
        //   included_tags: included_tags,
        // };

        // config.scopes.insert(scope_name.clone(), scope);
      }
    }

    Ok(config)
  }
}
