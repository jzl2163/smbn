use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn SetProcessWorkingSetSize(process: *mut c_void, minimum: usize, maximum: usize) -> i32;
}

/// Asks Windows to trim reclaimable pages after the UI has been hidden.
pub fn trim_working_set() {
    unsafe {
        let process = GetCurrentProcess();
        let _ = SetProcessWorkingSetSize(process, usize::MAX, usize::MAX);
    }
}
