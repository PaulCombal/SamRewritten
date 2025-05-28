use std::env;
use std::path::PathBuf;

pub fn get_executable_path() -> PathBuf {
    env::current_exe()
        .expect("Failed to get current executable path")
        .canonicalize() // Resolves symlinks to absolute path
        .expect("Failed to canonicalize path")
}

/// This function returns a valid directory where app data can be stored for a longer period of time.
#[inline]
#[cfg(target_os = "linux")]
pub fn get_app_cache_dir() -> String {
    if let Ok(snap_name) = env::var("SNAP_NAME") {
        if snap_name == "samrewritten" {
            return env::var("SNAP_USER_COMMON").unwrap_or(String::from("/tmp"));
        }

        // Most likely a dev config
        return ".".to_owned();
    }

    // Non-snap users
    "/tmp".to_owned()
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_app_cache_dir() -> String {
    todo!()
}

#[inline]
#[cfg(target_os = "linux")]
pub fn get_steamclient_lib_path() -> String {
    if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
        return real_home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
    }

    if let Ok(home) = env::var("HOME") {
        return home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so";
    }
    
    panic!("Failed to get Steam client library path");
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_steamclient_lib_path() -> String {
    todo!()
    // let program_files = std::env::var("ProgramFiles(x86)")?;
    // #[cfg(target_pointer_width = "64")]
    // let lib_steamclient_path = PathBuf::from(program_files + "\\Steam\\steamclient64.dll");
    // #[cfg(target_pointer_width = "32")]
    // let lib_steamclient_path = PathBuf::from(program_files + "\\Steam\\steamclient.dll");
    
    // Would it be better to get inspiration from this c# code?
    // Registry.GetValue(@"HKEY_LOCAL_MACHINE\Software\Valve\Steam", "InstallPath", null);
    // It would allow for multi-disk installs.
}

#[inline]
#[cfg(target_os = "linux")]
pub fn get_user_game_stats_schema_path(app_id: &u32) -> String {
    if let Ok(real_home) = env::var("SNAP_REAL_HOME") {
        return real_home + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_" + &app_id.to_string() + ".bin";
    }

    if let Ok(home) = env::var("HOME") {
        return home + "/snap/steam/common/.local/share/Steam/appcache/stats/UserGameStatsSchema_" + &app_id.to_string() + ".bin";
    }

    panic!("Failed to get User Game Stats Schema path");
}

#[inline]
#[cfg(target_os = "windows")]
pub fn get_user_game_stats_schema_path(app_id: &u32) -> String {
    // #[cfg(target_os = "windows")]
    // let program_files = env::var("ProgramFiles(x86)")?;
    // #[cfg(target_os = "windows")]
    // let bin_file = PathBuf::from(program_files + "\\Steam\\appcache\\stats\\UserGameStatsSchema_" + &self.app_id.to_string() + ".bin");
    todo!()
}