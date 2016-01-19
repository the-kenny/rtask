use std::io;
use libc;
use libc::c_ushort;

pub struct TerminalSize {
  columns: u16,
  rows: u16,
}

const DEFAULT_SIZE: TerminalSize = TerminalSize {
  columns: 80,
  rows: 24
};

pub fn terminal_size() -> TerminalSize {
  terminal_size_internal().unwrap_or(DEFAULT_SIZE)
}

// Ugly implementation details

#[repr(C)]
struct Winsize {
  ws_row: c_ushort,
  ws_col: c_ushort,
  ws_xpixel: c_ushort,
  ws_ypixel: c_ushort,
}

// This might be utterly wrong and have bad consequences on other
// platforms
#[cfg(unix)]
const TIOCGWINSZ: u64 = 0x5413;

fn terminal_size_internal() -> io::Result<TerminalSize> {
  let w: Winsize = Winsize {
    ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0
  };
  let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, TIOCGWINSZ, &w) };
  if ret == -1 {
    Err(io::Error::last_os_error())
  } else {
    Ok(TerminalSize {
      columns: w.ws_col,
      rows: w.ws_row,
    })
  }
}

#[test]
#[ignore]
fn test_terminal_size() {
  let ts = terminal_size_internal().unwrap();
  assert!(0 != ts.columns);
  assert!(0 != ts.rows);
}
