//! This crate exposes a single type: [`FileDesc`],
//! which acts as a thin wrapper around open file descriptors.
//!
//! The wrapped file descriptor is closed when the wrapper is dropped,
//! unless [`FileDesc::into_raw_fd()`] was called.
//!
//! A raw file descriptor can be wrapped directly using [`FileDesc::from_raw_fd()`],
//! or it can be duplicated and then wrapped using [`FileDesc::duplicate_raw_fd()`].
//! It is also possible to duplicate an already-wrapper file descriptor using [`FileDesc::duplicate()`].
//! If the platform supports it, all duplicated file descriptors are created with the `close-on-exec` flag set atomically,
//!
//! # Example
//! ```no_run
//! # fn main() -> std::io::Result<()> {
//! # let raw_fd = 10;
//! use filedesc::FileDesc;
//! let fd = unsafe { FileDesc::from_raw_fd(raw_fd) };
//! let duplicated = fd.duplicate()?;
//! assert_eq!(duplicated.get_close_on_exec()?, true);
//!
//! duplicated.set_close_on_exec(false)?;
//! assert_eq!(duplicated.get_close_on_exec()?, false);
//! # Ok(())
//! # }
//! ```


use std::os::unix::io::{RawFd, AsRawFd, IntoRawFd, FromRawFd};
use std::os::raw::c_int;

#[derive(Debug)]
/// Thin wrapper around an open file descriptor.
///
/// The wrapped file descriptor will be closed
/// when the wrapper is dropped.
pub struct FileDesc {
	fd: RawFd,
}

impl FileDesc {
	/// Wrap a raw file descriptor in a [`FileDesc`].
	///
	/// This does not do anything to the file descriptor.
	/// Notably, it does not set the `close-on-exec` flag.
	pub unsafe fn from_raw_fd(fd: RawFd) -> Self {
		Self { fd }
	}

	/// Duplicate a raw file descriptor and wrap it in a [`FileDesc`].
	pub unsafe fn duplicate_raw_fd(fd: RawFd) -> std::io::Result<Self> {
		// Try to dup with the CLOEXEC flag set.
		check_ret(libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0))
			.map(|raw| Self::from_raw_fd(raw))
			.or_else(|e| {
				// Fall back to setting CLOEXEC non-atomically.
				if e.raw_os_error() == Some(libc::EINVAL) {
					let fd = check_ret(libc::fcntl(fd, libc::F_DUPFD, 0))?;
					let fd = Self::from_raw_fd(fd);
					fd.set_close_on_exec(true)?;
					Ok(fd)
				} else {
					Err(e)
				}
			})
	}

	/// Get the raw file descriptor.
	///
	/// This function does not release ownership of the underlying file descriptor.
	/// The file descriptor will still be closed when the [`FileDesc`] is dropped.
	pub fn as_raw_fd(&self) -> RawFd {
		self.fd
	}

	/// Release and get the raw file descriptor.
	///
	/// This function releases ownership of the underlying file descriptor.
	/// The file descriptor will not be closed.
	pub fn into_raw_fd(self) -> RawFd {
		let fd = self.fd;
		std::mem::forget(self);
		fd
	}

	/// Try to duplicate the file descriptor.
	///
	/// The new file descriptor will have the `close-on-exec` flag set.
	/// If the platform supports it, the flag will be set atomically.
	pub fn duplicate(&self) -> std::io::Result<Self> {
		unsafe {
			Self::duplicate_raw_fd(self.as_raw_fd())
		}
	}

	/// Change the close-on-exec flag of the file descriptor.
	///
	/// Note that you should always try to create file descriptors with the close-on-exec flag already set atomically.
	/// Setting the flag later on introduces a race condition if another thread forks before the call to `set_close_on_exec` finishes.
	pub fn set_close_on_exec(&self, close_on_exec: bool) -> std::io::Result<()> {
		unsafe {
			// TODO: Are there platforms where we need to preserve other bits?
			let arg = if close_on_exec { libc::FD_CLOEXEC } else { 0 };
			check_ret(libc::fcntl(self.fd, libc::F_SETFD, arg))?;
			Ok(())
		}
	}

	/// Check the close-on-exec flag of the file descriptor.
	pub fn get_close_on_exec(&self) -> std::io::Result<bool> {
		unsafe {
			let ret = check_ret(libc::fcntl(self.fd, libc::F_GETFD, 0))?;
			Ok(ret & libc::FD_CLOEXEC != 0)
		}
	}
}

impl Drop for FileDesc {
	fn drop(&mut self) {
		if self.fd >= 0 {
			unsafe {
				libc::close(self.fd);
			}
		}
	}
}

impl FromRawFd for FileDesc {
	unsafe fn from_raw_fd(fd: RawFd) -> Self {
		Self::from_raw_fd(fd)
	}
}

impl AsRawFd for FileDesc {
	fn as_raw_fd(&self) -> RawFd {
		self.as_raw_fd()
	}
}

impl AsRawFd for &'_ FileDesc {
	fn as_raw_fd(&self) -> RawFd {
		(*self).as_raw_fd()
	}
}

impl IntoRawFd for FileDesc {
	fn into_raw_fd(self) -> RawFd {
		self.into_raw_fd()
	}
}

/// Wrap the return value of a libc function in an [`std::io::Result`].
///
/// If the return value is -1, [`last_os_error()`](std::io::Error::last_os_error) is returned.
/// Otherwise, the return value is returned wrapped as [`Ok`].
fn check_ret(ret: c_int) -> std::io::Result<c_int> {
	if ret == -1 {
		Err(std::io::Error::last_os_error())
	} else {
		Ok(ret)
	}
}

#[cfg(test)]
mod test;
