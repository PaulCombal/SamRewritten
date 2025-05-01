use std::os::raw::{c_int, c_char};
use crate::steam_client::types::AppId_t;

// You need to be whitelisted by Valve to use this interface.
// This is simply in the codebase for reference. 
// For tinkerers around, I've read that Spacewar is a whitelisted app.

#[repr(C)]
pub struct ISteamAppListVTable {
    pub get_num_installed_apps: unsafe extern "C" fn(*mut ISteamAppList) -> u32,
    pub get_installed_apps: unsafe extern "C" fn(
        *mut ISteamAppList,
        *mut AppId_t,
        u32
    ) -> u32,
    pub get_app_name: unsafe extern "C" fn(
        *mut ISteamAppList,
        AppId_t,
        *mut c_char,
        c_int
    ) -> c_int,
    pub get_app_install_dir: unsafe extern "C" fn(
        *mut ISteamAppList,
        AppId_t,
        *mut c_char,
        c_int
    ) -> c_int,
    pub get_app_build_id: unsafe extern "C" fn(
        *mut ISteamAppList,
        AppId_t
    ) -> c_int,
}

#[repr(C)]
pub struct ISteamAppList {
    pub vtable: *const ISteamAppListVTable,
}

pub const STEAMAPPLIST_INTERFACE_VERSION: &str = "STEAMAPPLIST_INTERFACE_VERSION001\0";