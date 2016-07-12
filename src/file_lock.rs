use libc;

use std::path::Path;
use std::io;
use std::ffi::CString;
use std::os::unix::ffi::OsStringExt;

pub struct Lock {
  fd: libc::c_int,
}

impl Lock {
  pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
    // TODO: Simplify path conversion if switching to nightly
    let path = CString::new(path.as_ref()
                            .as_os_str()
                            .to_os_string()
                            .into_vec())
      .unwrap();
    let flags = libc::O_RDWR | libc::O_CREAT;
    let fd = unsafe { libc::open(path.as_ptr(), flags, libc::S_IRWXU as libc::c_int) };
    if fd == -1 { return Err(io::Error::last_os_error()); } 

    let lock_result = unsafe { libc::flock(fd, libc::LOCK_EX) };
    if lock_result == -1 {
      Err(io::Error::last_os_error())
    } else {
      Ok(Lock { fd: fd })
    }
  }
}

impl Drop for Lock {
  fn drop(&mut self) {
    unsafe { libc::close(self.fd); }
  }
}
