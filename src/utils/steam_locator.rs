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

        if let Ok(path_str) = std::env::var("SAM_STEAMCLIENT_PATH") {
            return Some(Path::new(&path_str).to_owned());
        }

        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let path_str =
                real_home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
            return Some(Path::new(&path_str).to_owned());
        }

        let steam_install_paths: Vec<PathBuf> = Self::get_local_steam_install_root_folders()
            .into_iter()
            .map(|path| path.join("linux64/steamclient.so"))
            .filter(|path| path.exists())
            .collect();

        let first_path = steam_install_paths.first()?;

        if !silent && steam_install_paths.len() > 1 {
            eprintln!("[STEAM LOCATOR] Found multiple Steam installations. Using the first one.");
            for path in &steam_install_paths {
                eprintln!("[STEAM LOCATOR] - {}", path.display());
            }
        }

        Some(first_path.clone())
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
        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let full_path = real_home
                + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_";
            return Some(full_path);
        }

        let dirs = Self::get_local_steam_install_root_folders();

        if dirs.is_empty() {
            return None;
        }

        Some(dirs[0].to_str()?.to_owned() + "/appcache/stats/UserGameStatsSchema_")
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
    pub fn get_local_steam_install_root_folders() -> Vec<PathBuf> {
        use std::path::PathBuf;

        if let Ok(real_home) = std::env::var("SNAP_REAL_HOME") {
            let prefix = PathBuf::from(real_home).join("snap/steam/common/.local/share/Steam");
            return vec![prefix];
        }

        if let Ok(path) = std::env::var("SAM_STEAM_INSTALL_ROOT") {
            return vec![PathBuf::from(path)];
        }

        let home = std::env::var("HOME").expect("Failed to get home dir");
        let home_path = PathBuf::from(home);

        let potential_dirs = [
            home_path.join("snap/steam/common/.local/share/Steam"),
            home_path.join(".local/share/Steam"),
            home_path.join(".steam/steam"),
            home_path.join(".steam/debian-installation"),
            home_path.join(".steam/root"),
        ];

        potential_dirs
            .into_iter()
            .filter(|path| path.exists() && !path.is_symlink())
            .collect()
    }

    #[cfg(target_os = "linux")]
    pub fn get_local_app_banner_file_prefix() -> Option<String> {
        let dirs = Self::get_local_steam_install_root_folders();

        if dirs.is_empty() {
            None
        } else {
            Some(dirs[0].to_str()?.to_owned() + "/appcache/librarycache/")
        }
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
