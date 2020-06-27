# filedesc [![docs][docs-badge]][docs] [![tests][tests-badge]][tests]
[docs]: https://docs.rs/filedesc/
[tests]: https://github.com/de-vri-es/filedesc-rs/actions?query=workflow%3Atests
[docs-badge]: https://docs.rs/filedesc/badge.svg
[tests-badge]: https://github.com/de-vri-es/filedesc-rs/workflows/tests/badge.svg

This crate exposes a single type: [`FileDesc`][FileDesc],
which acts as a thin wrapper around open file descriptors.

The wrapped file descriptor is closed when the wrapper is dropped,
unless [`FileDesc::into_raw_fd()`][into_raw_fd] was called.

A raw file descriptor can be wrapper directly using [`FileDesc::from_raw()`][from_raw_fd],
or it can be duplicated and then wrapped using [`FileDesc::duplicate_raw_fd()`][duplicate_raw_fd].
It is also possible to duplicate an already-wrapper file descriptor using [`FileDesc::duplicate()`][duplicate].
If the platform supports it, all duplicated file descriptors are created with the `close-on-exec` flag set atomically,

[FileDesc]: https://docs.rs/filedesc/latest/filedesc/struct.FileDesc.html
[into_raw_fd]: https://docs.rs/filedesc/latest/filedesc/struct.FileDesc.html#method.into_raw_fd
[from_raw_fd]: https://docs.rs/filedesc/latest/filedesc/struct.FileDesc.html#method.from_raw_fd
[duplicate_raw_fd]: https://docs.rs/filedesc/latest/filedesc/struct.FileDesc.html#method.duplicate_raw_fd
[duplicate]: https://docs.rs/filedesc/latest/filedesc/struct.FileDesc.html#method.duplicate

## Example
```rust
use filedesc::FileDesc;
let fd = unsafe { FileDesc::from_raw_fd(raw_fd) };
let duplicated = fd.duplicate()?;
assert_eq!(duplicated.get_close_on_exec()?, true);

duplicated.set_close_on_exec(false)?;
assert_eq!(duplicated.get_close_on_exec()?, false);
```
