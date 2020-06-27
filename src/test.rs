use crate::FileDesc;
use assert2::assert;

#[test]
fn test_get_close_on_exec() {
	let fd = unsafe { FileDesc::duplicate_raw_fd(2i32).unwrap() };
	assert!(let Ok(true) = fd.get_close_on_exec());
	assert!(let Ok(()) = fd.set_close_on_exec(false));
	assert!(let Ok(false) = fd.get_close_on_exec());
	assert!(let Ok(_) = fd.duplicate());
}
