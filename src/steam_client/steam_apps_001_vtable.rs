use std::os::raw::{c_char, c_int};
use crate::steam_client::types::AppId_t;

#[repr(C)]
pub struct ISteamApps001VTable {
    pub get_app_data: unsafe extern "C" fn(
        *mut ISteamApps001,
        AppId_t,
        *const c_char,
        *mut c_char,
        c_int
    ) -> c_int,
}

// The main interface structure
#[repr(C)]
pub struct ISteamApps001 {
    pub vtable: *const ISteamApps001VTable,
}

// Interface version constant
pub const STEAMAPPS001_INTERFACE_VERSION: &str = "STEAMAPPS_INTERFACE_VERSION001\0";