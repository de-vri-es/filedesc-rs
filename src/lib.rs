//! This crate exposes a single type: [`FileDesc`][FileDesc],
//! which acts as a thin wrapper around open file descriptors.
//! The wrapped file descriptor is closed when the wrapper is dropped.
//!
//! You can call [`FileDesc::new()`][FileDesc::new] with any type that implements [`IntoRawFd`][IntoRawFd],
//! or duplicate the file descriptor of a type that implements [`AsRawFd`][AsRawFd] with [`duplicate_from`][FileDesc::duplicate_from].
//!
//! The same is possible for raw file descriptors with the `unsafe` [`from_raw_fd()`][FileDesc::from_raw_fd] and [`duplicate_raw_fd()`][FileDesc::duplicate_raw_fd].
//! Wrapped file descriptors can also be duplicated with the [`duplicate()`][FileDesc::duplicate] function.
//!
//! # Close-on-exec
//! Whenever the library duplicates a file descriptor, it tries to set the `close-on-exec` flag atomically.
//! On platforms where this is not supported, the library falls back to setting the flag non-atomically.
//! When an existing file descriptor is wrapped, the `close-on-exec` flag is left as it was.
//!
//! You can also check or set the `close-on-exec` flag with the [`get_close_on_exec()`][FileDesc::get_close_on_exec]
//! and [`set_close_on_exec`][FileDesc::set_close_on_exec] functions.
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

use std::os::raw::c_int;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// If false, skip attempting to duplicate with F_DUPFD_CLOEXEC fcntl.
///
/// Used to reduce the number of syscalls on platforms that don't support it.
static TRY_DUPFD_CLOEXEC: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
/// Thin wrapper around an open file descriptor.
///
/// The wrapped file descriptor will be closed
/// when the wrapper is dropped.
pub struct FileDesc {
	fd: RawFd,
}

impl FileDesc {
	/// Create [`FileDesc`] from an object that owns a file descriptor.
	///
	/// This does not do anything to the file descriptor other than wrapping it.
	/// Notably, it does not set the `close-on-exec` flag.
	pub fn new<T: IntoRawFd>(fd: T) -> Self {
		let fd = fd.into_raw_fd();
		Self { fd }
	}

	/// Wrap a raw file descriptor in a [`FileDesc`].
	///
	/// This does not do anything to the file descriptor other than wrapping it.
	/// Notably, it does not set the `close-on-exec` flag.
	///
	/// # Safety
	/// The input must be a valid file descriptor.
	/// The file descriptor must not be closed other than by the created [`FileDesc`],
	/// unless ownership of the file descriptor is relinquished by calling [`into_raw_fd()`](Self::into_raw_fd).
	pub unsafe fn from_raw_fd(fd: RawFd) -> Self {
		Self { fd }
	}

	/// Duplicate a file descriptor from an object that has a file descriptor.
	///
	/// The new file descriptor will have the `close-on-exec` flag set.
	/// If the platform supports it, the flag will be set atomically.
	pub fn duplicate_from<T: AsRawFd>(other: &T) -> std::io::Result<Self> {
		unsafe { Self::duplicate_raw_fd(other.as_raw_fd()) }
	}

	/// Duplicate a raw file descriptor and wrap it in a [`FileDesc`].
	///
	/// The new file descriptor will have the `close-on-exec` flag set.
	/// If the platform supports it, the flag will be set atomically.
	///
	/// # Safety
	/// The input must be a valid file descriptor.
	/// The file descriptor must not be closed other than by the created [`FileDesc`],
	/// unless ownership of the file descriptor is relinquished by calling [`into_raw_fd()`](Self::into_raw_fd).
	pub unsafe fn duplicate_raw_fd(fd: RawFd) -> std::io::Result<Self> {
		// Try to dup with the close-on-exec flag set.
		if TRY_DUPFD_CLOEXEC.load(Relaxed) {
			match check_ret(libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0)) {
				Err(ref e) if e.raw_os_error() == Some(libc::EINVAL) => {
					TRY_DUPFD_CLOEXEC.store(false, Relaxed);
				},
				Ok(x) => return Ok(Self::from_raw_fd(x)),
				Err(e) => return Err(e),
			}
		}

		// Fall back to setting close-on-exec non-atomically.
		let fd = check_ret(libc::fcntl(fd, libc::F_DUPFD, 0))?;
		let fd = Self::from_raw_fd(fd);
		fd.set_close_on_exec(true)?;
		Ok(fd)
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
		unsafe { Self::duplicate_raw_fd(self.as_raw_fd()) }
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
