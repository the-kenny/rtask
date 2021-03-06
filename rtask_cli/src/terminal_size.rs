#![allow(dead_code)]

use libc;
use libc::c_ushort;
use std::io;

pub struct TerminalSize {
    pub columns: usize,
    pub rows: usize,
}

pub fn terminal_size() -> TerminalSize {
    terminal_size_internal().unwrap_or(DEFAULT_SIZE)
}

const DEFAULT_SIZE: TerminalSize = TerminalSize {
    columns: 80,
    rows: 24,
};

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

#[derive(Debug, Fail)]
enum TerminalSizeError {
    #[fail(display = "IO Error: {}", _0)]
    Io(io::Error),
    #[fail(display = "Terminal size returned zero value for width or height")]
    ZeroSize,
}

impl From<io::Error> for TerminalSizeError {
    fn from(err: io::Error) -> Self {
        TerminalSizeError::Io(err)
    }
}

fn terminal_size_internal() -> Result<TerminalSize, TerminalSizeError> {
    let w: Winsize = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, TIOCGWINSZ, &w) };
    if ret == -1 {
        let last_err = io::Error::last_os_error();
        warn!("Got OS Error: {:?}", last_err);
        Err(last_err.into())
    } else {
        if w.ws_col > 0 && w.ws_row > 0 {
            info!("Got terminal size: {}x{}", w.ws_col, w.ws_row);
            Ok(TerminalSize {
                columns: w.ws_col as usize,
                rows: w.ws_row as usize,
            })
        } else {
            Err(TerminalSizeError::ZeroSize)
        }
    }
}

#[test]
#[ignore]
fn test_terminal_size() {
    let ts = terminal_size_internal().unwrap();
    assert!(0 != ts.columns);
    assert!(0 != ts.rows);
}
