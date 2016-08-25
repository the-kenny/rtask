use std::io;

use ::task::StringExt;

pub fn print_table<'a, S: AsRef<str>>(writer: &mut io::Write, rows: Vec<Vec<S>>) -> io::Result<()> {
  let mut widths = vec![0; rows[0].len()];
  for row in rows.iter() {
    for (n, field) in row.iter().enumerate() {
      use std::cmp;
      widths[n] = cmp::max(widths[n], field.as_ref().len());
    }
  }

  // TODO: Implement max. terminal size handling

  for row in rows {
    try!(write!(writer, " "));

    for (n, text) in row.iter().enumerate() {
      let width = widths[n];
      let text = text.as_ref();
      try!(write!(writer, "{0:>1$} ", text.ellipsize(width), width));
    }

    try!(write!(writer, "\n"));
  }

  Ok(())
}

#[test]
fn test() {
  print_table(&mut io::stdout(),
              vec![vec!["1", "some long description foo bar baz"],
                   vec!["2", "short desc"]]);
}
