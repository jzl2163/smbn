use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub exe_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> io::Result<Self> {
        let exe = env::current_exe()?;
        let exe_dir = exe.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let portable = exe_dir.join("portable.flag").is_file();
        let data_dir = if portable {
            exe_dir.join("data")
        } else {
            env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| exe_dir.clone())
                .join("Smbn")
        };
        let log_dir = data_dir.join("logs");
        fs::create_dir_all(&log_dir)?;
        Ok(Self {
            exe_dir,
            config_file: data_dir.join("config.json"),
            data_dir,
            log_dir,
        })
    }

    pub fn engine_executable(&self) -> Option<EngineLaunch> {
        let candidates = [
            self.exe_dir.join("engine").join("Smbn.Engine.exe"),
            self.exe_dir.join("Smbn.Engine.exe"),
        ];
        for path in candidates {
            if path.is_file() {
                return Some(EngineLaunch::Executable(path));
            }
        }

        let dll_candidates = [
            self.exe_dir.join("engine").join("Smbn.Engine.dll"),
            self.exe_dir.join("Smbn.Engine.dll"),
        ];
        for path in dll_candidates {
            if path.is_file() {
                return Some(EngineLaunch::DotnetDll(path));
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum EngineLaunch {
    Executable(PathBuf),
    DotnetDll(PathBuf),
}
