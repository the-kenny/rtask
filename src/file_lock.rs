use libc;

use std::fmt::Debug;
use std::path::Path;
use std::{fs, io};
use std::path::PathBuf;
use std::ffi::CString;
use std::os::unix::ffi::OsStringExt;

pub struct FileLock {
  fd: libc::c_int,
  path: PathBuf,
  pub delete_on_drop: bool,
}

impl FileLock {
  pub fn new<P>(path: P) -> io::Result<Self>
    where P: AsRef<Path> + Debug {
    debug!("Acquiring FileLock on {:?}", path);
    let cpath = CString::new(path.as_ref()
                            .as_os_str()
                            .to_os_string()
                            .into_vec())
      .unwrap();
    let flags = libc::O_RDWR | libc::O_CREAT;
    let fd = unsafe { libc::open(cpath.as_ptr(), flags, libc::S_IRWXU as libc::c_int) };
    if fd == -1 { return Err(io::Error::last_os_error()); } 

    let lock_result = unsafe { libc::flock(fd, libc::LOCK_EX) };
    if lock_result == -1 {
      Err(io::Error::last_os_error())
    } else {
      Ok(FileLock {
        fd: fd,
        path: path.as_ref().to_path_buf(),
        delete_on_drop: false
      })
    }
  }
}

impl Drop for FileLock {
  fn drop(&mut self) {
    unsafe { libc::close(self.fd); }

    if self.delete_on_drop {
      fs::remove_file(&self.path).expect("Failed to remove pid file");
    }
  }
}
