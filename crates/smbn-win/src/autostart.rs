use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

const HKEY_CURRENT_USER: usize = 0x80000001;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(key: usize, sub_key: *const u16, options: u32, sam: u32, result: *mut usize) -> i32;
    fn RegSetValueExW(key: usize, value_name: *const u16, reserved: u32, kind: u32, data: *const u8, data_len: u32) -> i32;
    fn RegDeleteValueW(key: usize, value_name: *const u16) -> i32;
    fn RegCloseKey(key: usize) -> i32;
}

pub fn set_enabled(enabled: bool, executable: &Path) -> io::Result<()> {
    let sub_key = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value_name = wide("Smbn");
    let mut key = 0usize;
    let result = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_SET_VALUE, &mut key) };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result));
    }

    let operation_result = if enabled {
        let command = format!("\"{}\" --minimized", executable.display());
        let data = wide(&command);
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, value_name.as_ptr()) }
    };
    unsafe { RegCloseKey(key) };

    // ERROR_FILE_NOT_FOUND when disabling an already-disabled value is harmless.
    if operation_result == ERROR_SUCCESS || (!enabled && operation_result == 2) {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(operation_result))
    }
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value).encode_wide().chain(Some(0)).collect()
}
