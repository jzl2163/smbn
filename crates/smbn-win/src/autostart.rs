use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

type HKey = isize;

// Win32 defines predefined HKEY values by casting the 32-bit signed LONG value
// through LONG_PTR. Preserve that sign extension on 64-bit Windows.
const HKEY_CURRENT_USER: HKey = 0x8000_0001u32 as i32 as isize;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_SZ: u32 = 1;
const ERROR_SUCCESS: i32 = 0;

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        key: HKey,
        sub_key: *const u16,
        options: u32,
        sam: u32,
        result: *mut HKey,
    ) -> i32;
    fn RegSetValueExW(
        key: HKey,
        value_name: *const u16,
        reserved: u32,
        kind: u32,
        data: *const u8,
        data_len: u32,
    ) -> i32;
    fn RegDeleteValueW(key: HKey, value_name: *const u16) -> i32;
    fn RegCloseKey(key: HKey) -> i32;
}

pub fn set_enabled(enabled: bool, executable: &Path) -> io::Result<()> {
    let sub_key = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value_name = wide("Smbn");
    let mut key: HKey = 0;
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
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
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::HKEY_CURRENT_USER;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn predefined_hkey_is_sign_extended() {
        assert_eq!(HKEY_CURRENT_USER as u64, 0xffff_ffff_8000_0001);
    }
}
