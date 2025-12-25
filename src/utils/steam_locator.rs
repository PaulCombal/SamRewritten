use crate::utils::ipc_types::SamError;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

pub struct SteamLocator {
    lib_path: Option<PathBuf>,
    user_game_stats_schema_prefix: Option<String>,
    local_app_banner_file_prefix: Option<String>,
}

impl SteamLocator {
    pub fn new() -> Self {
        Self {
            lib_path: None,
            user_game_stats_schema_prefix: None,
            local_app_banner_file_prefix: None,
        }
    }

    pub fn global() -> &'static RwLock<SteamLocator> {
        static INSTANCE: OnceLock<RwLock<SteamLocator>> = OnceLock::new();
        INSTANCE.get_or_init(|| RwLock::new(SteamLocator::new()))
    }

    pub fn get_lib_path(&mut self, silent: bool) -> Option<PathBuf> {
        if self.lib_path.is_none() {
            self.lib_path = Self::get_steamclient_lib_path(silent);
        }
        self.lib_path.clone()
    }

    pub fn get_user_game_stats_schema(&mut self, app_id: &u32) -> Result<PathBuf, SamError> {
        if self.user_game_stats_schema_prefix.is_none() {
            self.user_game_stats_schema_prefix = Self::get_user_game_stats_schema_prefix();
        }

        if let Some(prefix) = self.user_game_stats_schema_prefix.as_ref() {
            let path_str = format!("{}{}.bin", prefix, app_id);
            return Ok(PathBuf::from(path_str));
        }

        Err(SamError::UnknownError)
    }

    pub fn get_local_app_banner_file_path(&mut self, app_id: &u32) -> Result<PathBuf, SamError> {
        if self.local_app_banner_file_prefix.is_none() {
            self.local_app_banner_file_prefix = Self::get_local_app_banner_file_prefix();
        }

        if let Some(prefix) = self.local_app_banner_file_prefix.as_ref() {
            let path_str = format!("{}{}/header.jpg", prefix, app_id);
            return Ok(PathBuf::from(path_str));
        }

        Err(SamError::UnknownError)
    }

    #[cfg(target_os = "linux")]
    pub fn get_steamclient_lib_path(silent: bool) -> Option<PathBuf> {
        use std::path::Path;

        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let path_str =
                real_home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
            return Some(Path::new(&path_str).to_owned());
        }

        let home = std::env::var("HOME").expect("Failed to get home dir");
        let lib_paths = [
            home.clone() + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so",
            home.clone() + "/.steam/debian-installation/linux64/steamclient.so",
            home.clone() + "/.steam/sdk64/steamclient.so",
            home.clone() + "/.steam/steam/linux64/steamclient.so",
            home.clone() + "/.local/share/Steam/linux64/steamclient.so",
            home + "/.steam/root/linux64/steamclient.so",
        ];

        if silent {
            for lib_path in lib_paths {
                let path = Path::new(&lib_path);
                if path.exists() {
                    return Some(path.into());
                }
            }

            return None;
        }

        let mut found_paths: Vec<PathBuf> = vec![];
        for lib_path in lib_paths {
            let path = Path::new(&lib_path);
            if path.exists() {
                found_paths.push(path.into());
            }
        }

        if found_paths.is_empty() {
            return None;
        }

        if found_paths.len() > 1 {
            eprintln!("[STEAM LOCATOR] Found multiple Steam installations. Using the first one.");
            for path in found_paths.iter() {
                eprintln!("[STEAM LOCATOR] - {}", path.display());
            }
        }

        Some(found_paths[0].clone())
    }

    #[cfg(target_os = "windows")]
    pub fn get_steamclient_lib_path(_silent: bool) -> Option<PathBuf> {
        use std::path::PathBuf;
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

        const REG_PATH: &str = "SOFTWARE\\Valve\\Steam";
        const VALUE_NAME: &str = "SteamPath";

        // Try HKEY_CURRENT_USER first
        if let Ok(subkey) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(REG_PATH) {
            if let Ok(value) = subkey.get_value::<String, _>(VALUE_NAME) {
                let path = PathBuf::from(value).join("steamclient64.dll");
                return Some(path);
            }
        }

        // Fallback to HKEY_LOCAL_MACHINE
        if let Ok(subkey) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(REG_PATH) {
            if let Ok(value) = subkey.get_value::<String, _>(VALUE_NAME) {
                let path = PathBuf::from(value).join("steamclient64.dll");
                return Some(path);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn get_user_game_stats_schema_prefix() -> Option<String> {
        use std::path::Path;

        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let full_path = real_home
                + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_";
            return Some(full_path);
        }

        let home = std::env::var("HOME").expect("Failed to get home dir");
        let install_dirs = [
            home.clone() + "/snap/steam/common/.local/share/Steam",
            home.clone() + "/.steam/debian-installation",
            home.clone() + "/.steam/steam",
            home.clone() + "/.local/share/Steam",
            home + "/.steam/root",
        ];

        for install_dir in install_dirs {
            if Path::new(&install_dir).exists() {
                return Some(install_dir + "/appcache/stats/UserGameStatsSchema_");
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    pub fn get_user_game_stats_schema_prefix() -> Option<String> {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};

        let steam_key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .ok()?;

        let steam_path: String = steam_key.get_value("SteamPath").ok()?;

        Some(steam_path + "/appcache/stats/UserGameStatsSchema_")
    }

    #[cfg(target_os = "linux")]
    pub fn get_local_app_banner_file_prefix() -> Option<String> {
        use std::path::Path;

        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let prefix = real_home + "/snap/steam/common/.local/share/Steam/appcache/librarycache/";
            return Some(prefix);
        }

        let home = std::env::var("HOME").expect("Failed to get home dir");
        let install_dirs = [
            home.clone() + "/snap/steam/common/.local/share/Steam",
            home.clone() + "/.steam/debian-installation",
            home.clone() + "/.steam/steam",
            home.clone() + "/.local/share/Steam",
            home + "/.steam/root",
        ];

        for install_dir in install_dirs {
            if Path::new(&install_dir).exists() {
                return Some(install_dir + "/appcache/librarycache/");
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    pub fn get_local_app_banner_file_prefix() -> Option<String> {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;

        let subkey = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .ok()?;

        let value = subkey.get_value::<String, &'static str>("SteamPath").ok()?;

        Some(value + "/appcache/librarycache/")
    }
}
