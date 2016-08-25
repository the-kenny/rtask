use std::io;

use ::task::StringExt;

pub struct TablePrinter<'a, S: AsRef<str>> {
  titles: Vec<&'a str>,
  widths: Vec<usize>,

  pub rows: Vec<Vec<S>>,
}

impl<'a, S: AsRef<str>> TablePrinter<'a, S> {
  pub fn new() -> Self {
    TablePrinter {
      titles: vec![],
      widths: vec![],
      rows:   vec![],
    }
  }

  pub fn add_column(&mut self, name: &'a str) {
    self.titles.push(name);
    self.widths.push(name.len());
  }

  pub fn calculate_widths(&mut self) {
    for row in self.rows.iter() { assert_eq!(self.titles.len(), row.len()); }

    for row in self.rows.iter() {
      for (n, field) in row.iter().enumerate() {
        use std::cmp;
        self.widths[n] = cmp::max(self.widths[n], field.as_ref().len());
      }
    }

    debug!("calculated widths: {:?}", self.widths);
  }

  pub fn print(&self, writer: &mut io::Write) -> io::Result<()> {
    // TODO: Implement max. terminal size handling

    for (title, width) in self.titles.iter().zip(self.widths.iter()) {
      try!(write!(writer, " {0:^1$} ", title, width));
    }
    try!(write!(writer, "\n"));

    for row in self.rows.iter() {
      for (n, text) in row.iter().enumerate() {
        let width = self.widths[n];
        let text = text.as_ref();
        try!(write!(writer, " {0:>1$} ", text.ellipsize(width), width));
      }

      try!(write!(writer, "\n"));
    }

    Ok(())
  }

}

// pub fn print_table<'a, S: AsRef<str>>(writer: &mut io::Write, rows: Vec<Vec<S>>) -> io::Result<()> {
//   let mut widths = vec![0; rows[0].len()];
//   for row in rows.iter() {
//     for (n, field) in row.iter().enumerate() {
//       use std::cmp;
//       widths[n] = cmp::max(widths[n], field.as_ref().len());
//     }
//   }

//   // TODO: Implement max. terminal size handling

//   for row in rows {
//     try!(write!(writer, " "));

//     for (n, text) in row.iter().enumerate() {
//       let width = widths[n];
//       let text = text.as_ref();
//       try!(write!(writer, "{0:>1$} ", text.ellipsize(width), width));
//     }

//     try!(write!(writer, "\n"));
//   }

//   Ok(())
// }

#[test]
fn test() {

  let mut t = TablePrinter::new();
  t.add_column("ID");
  t.add_column("Description");
  t.rows = vec![vec!["1", "some long description foo bar baz"],
                vec!["2", "short desc"]];
  t.calculate_widths();

  t.print(&mut io::stdout()).unwrap();
}
