use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::ffi::c_void;
use std::io;
use std::ptr::null_mut;
use zeroize::Zeroize;

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;
const DESCRIPTION: &[u16] = &[
    'S' as u16, 'M' as u16, 'B' as u16, 'N' as u16, 0,
];
const ENTROPY: &[u8] = b"Smbn/DPAPI/v1";

#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        data_in: *const DataBlob,
        description: *const u16,
        optional_entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt_struct: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;

    fn CryptUnprotectData(
        data_in: *const DataBlob,
        description: *mut *mut u16,
        optional_entropy: *const DataBlob,
        reserved: *mut c_void,
        prompt_struct: *mut c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}

pub fn protect(plaintext: &str) -> Result<String, DpapiError> {
    let mut bytes = plaintext.as_bytes().to_vec();
    let input = DataBlob { cb_data: bytes.len() as u32, pb_data: bytes.as_mut_ptr() };
    let entropy = DataBlob { cb_data: ENTROPY.len() as u32, pb_data: ENTROPY.as_ptr() as *mut u8 };
    let mut output = DataBlob { cb_data: 0, pb_data: null_mut() };
    let ok = unsafe {
        CryptProtectData(
            &input,
            DESCRIPTION.as_ptr(),
            &entropy,
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    bytes.zeroize();
    if ok == 0 {
        return Err(DpapiError::Windows(io::Error::last_os_error()));
    }
    let protected = unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize) };
    let encoded = STANDARD.encode(protected);
    unsafe { LocalFree(output.pb_data.cast()) };
    Ok(encoded)
}

pub fn unprotect(encoded: &str) -> Result<String, DpapiError> {
    let mut protected = STANDARD.decode(encoded).map_err(DpapiError::Base64)?;
    let input = DataBlob { cb_data: protected.len() as u32, pb_data: protected.as_mut_ptr() };
    let entropy = DataBlob { cb_data: ENTROPY.len() as u32, pb_data: ENTROPY.as_ptr() as *mut u8 };
    let mut output = DataBlob { cb_data: 0, pb_data: null_mut() };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            null_mut(),
            &entropy,
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    protected.zeroize();
    if ok == 0 {
        return Err(DpapiError::Windows(io::Error::last_os_error()));
    }
    let clear = unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize) };
    let result = String::from_utf8(clear.to_vec()).map_err(DpapiError::Utf8);
    unsafe {
        std::ptr::write_bytes(output.pb_data, 0, output.cb_data as usize);
        LocalFree(output.pb_data.cast());
    }
    result
}

#[derive(Debug, thiserror::Error)]
pub enum DpapiError {
    #[error("DPAPI 调用失败: {0}")]
    Windows(io::Error),
    #[error("密码密文 Base64 无效: {0}")]
    Base64(base64::DecodeError),
    #[error("解密后的密码不是 UTF-8: {0}")]
    Utf8(std::string::FromUtf8Error),
}
