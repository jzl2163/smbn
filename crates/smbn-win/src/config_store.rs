use crate::paths::AppPaths;
use smbn_core::AppConfig;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

#[link(name = "kernel32")]
extern "system" {
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
}

pub fn load(paths: &AppPaths) -> Result<AppConfig, ConfigStoreError> {
    if !paths.config_file.exists() {
        return Ok(AppConfig::default());
    }
    let file = File::open(&paths.config_file)?;
    let reader = BufReader::new(file);
    let mut config: AppConfig = serde_json::from_reader(reader)?;
    migrate(&mut config)?;
    Ok(config)
}

pub fn save(paths: &AppPaths, config: &AppConfig) -> Result<(), ConfigStoreError> {
    fs::create_dir_all(&paths.data_dir)?;
    let temp = paths.config_file.with_extension("json.tmp");
    let file = File::create(&temp)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, config)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);

    if paths.config_file.exists() {
        let backup = paths.config_file.with_extension("json.bak");
        fs::copy(&paths.config_file, backup)?;
    }
    replace_file(&temp, &paths.config_file)?;
    Ok(())
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    let source = wide(source.as_os_str());
    let destination = wide(destination.as_os_str());
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

fn migrate(config: &mut AppConfig) -> Result<(), ConfigStoreError> {
    match config.version {
        smbn_core::CURRENT_CONFIG_VERSION => Ok(()),
        version => Err(ConfigStoreError::UnsupportedVersion(version)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigStoreError {
    #[error("配置文件读写失败: {0}")]
    Io(#[from] io::Error),
    #[error("配置文件 JSON 无效: {0}")]
    Json(#[from] serde_json::Error),
    #[error("不支持配置版本 {0}")]
    UnsupportedVersion(u32),
}
