use task::{Tag, Task, Priority};

use std::str::FromStr;
use std::fmt;
use regex::Regex;

#[derive(PartialEq, Eq, Debug)]
pub enum Flag {
    Priority(Priority),
    TagPositive(Tag),
    TagNegative(Tag),
}

impl Flag {
    pub fn from_str<S: AsRef<str>>(s: S) -> Option<Flag> {
        lazy_static! {
            static ref PRIORITY_RE: Regex = Regex::new("^p(?:riority)?:(.+)$").unwrap();
            static ref TAG_POS_RE: Regex = Regex::new("^\\+(.+)$").unwrap();
            static ref TAG_NEG_RE: Regex = Regex::new("^-(.+)$").unwrap();
        }

        let s = s.as_ref();

        // TODO: Write a loop
        let priority = PRIORITY_RE
            .captures(s)
            .and_then(|cs| cs.get(1))
            .map(|m| m.as_str())
            .and_then(|s| Priority::from_str(s).ok())
            .map(Flag::Priority);

        let pos_tag = TAG_POS_RE
            .captures(s)
            .and_then(|cs| cs.get(1))
            .map(|m| m.as_str())
            .map(String::from)
            .map(Flag::TagPositive);

        let neg_tag = TAG_NEG_RE
            .captures(s)
            .and_then(|cs| cs.get(1))
            .map(|m| m.as_str())
            .map(String::from)
            .map(Flag::TagNegative);

        priority.or(pos_tag).or(neg_tag)
    }

    pub fn matches(&self, t: &Task) -> bool {
        use self::Flag::*;
        match *self {
            Priority(p) => t.priority == p,
            TagPositive(ref tag) => t.tags.contains(tag),
            TagNegative(ref tag) => !t.tags.contains(tag),
        }
    }

    pub fn apply_to(&self, t: &mut Task) {
        use self::Flag::*;
        match *self {
            Priority(p) => {
                t.priority = p;
            }
            TagPositive(ref tag) => {
                t.tags.insert(tag.clone());
            }
            TagNegative(ref tag) => {
                t.tags.remove(tag);
            }
        }
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Flag::*;
        match *self {
            Priority(ref p) => write!(f, "priority:{}", p),
            TagPositive(ref t) => write!(f, "+{}", t),
            TagNegative(ref t) => write!(f, "-{}", t),
        }
    }
}
