v0.3.0:
  * Add `new()` function that convert `IntoRawFd` objects.
  * Add `duplicate_from()` function that duplicates `AsRawFd` objects.
  * Remember if `F_DUPFD_CLOEXEC` is unsupported to avoid unnecessary syscalls.

v0.2.0:
  * Remove `check_ret()` from public API.

v0.1.0:
  * Initial release.
