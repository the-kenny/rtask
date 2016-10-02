use std::io;
use std::collections::HashMap;
pub use ansi_term::{Colour, Style};

use ::task::StringExt;

#[derive(Copy,Clone)]
pub enum Alignment {
  Left,
  Center,
  Right
}

pub struct TablePrinter {
  pub titles: Vec<&'static str>,
  pub alignments: HashMap<&'static str, Alignment>,
  pub width_limit: Option<usize>,
}

pub struct PrintRow {
  pub fields: Vec<String>,
  pub style: Option<Style>,
}

#[derive(Debug)]
pub enum PrintError {
  IO(io::Error),
  TerminalTooNarrow
}

impl From<io::Error> for PrintError {
  fn from(e: io::Error) -> Self {
    PrintError::IO(e)
  }
}

impl TablePrinter {
  pub fn new() -> Self {
    TablePrinter {
      titles: vec![],
      alignments: HashMap::new(),
      width_limit: None,
    }
  }

  pub fn print(&self, writer: &mut io::Write, rows: &[PrintRow]) -> Result<(), PrintError> {
    // TODO: Implement "dumb" output for dumb terminals

    let mut widths: Vec<usize> = self.titles.iter().map(|x| x.len()).collect();
    for row in rows.iter() {
      for (n, field) in row.fields.iter().enumerate() {
        use std::cmp;
        widths[n] = cmp::max(widths[n], field.len());
      }
    }

    if let Some(width_limit) = self.width_limit {
      let max_width: usize = widths.iter().sum();
      if max_width >= width_limit {
        return Err(PrintError::TerminalTooNarrow)
      }
    }

    let header_style = Style::default().bold().underline();
    for (title, width) in self.titles.iter().zip(widths.iter()) {
      // TODO: Report this issues to ansi_term
      // TODO: styled_title and styled_title.to_string() have different .len()?!
      let styled_title = header_style.paint(*title).to_string();
      let diff = styled_title.len() - title.len();
      try!(write!(writer, " {0:^1$} ", styled_title, width+diff));
    }
    try!(write!(writer, "\n"));

    for row in rows.iter() {
      let style = row.style.unwrap_or(Style::default());

      for (n, (text, title)) in row.fields.iter().zip(self.titles.iter()).enumerate() {
        let width = widths[n];
        let alignment = self.alignments.get(title).map(|a| *a).unwrap_or(Alignment::Center);

        use self::Alignment::*;
        let line = match alignment {
          Left   => format!(" {0:<1$} ", text.ellipsize(width), width),
          Center => format!(" {0:^1$} ", text.ellipsize(width), width),
          Right  => format!(" {0:>1$} ", text.ellipsize(width), width),
        };
        try!(write!(writer, "{}", style.paint(line)));
      }

      try!(write!(writer, "\n"));
    }

    Ok(())
  }
}
