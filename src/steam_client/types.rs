use std::os::raw::{c_char, c_int, c_void};
use crate::steam_client::steam_client_vtable::ISteamClient;

#[allow(non_camel_case_types)]
pub type AppId_t = u32;

#[allow(non_camel_case_types)]
pub type DepotId_t = u32;

#[allow(non_camel_case_types)]
pub type SteamAPICall_t = u64;

pub type HSteamPipe = c_int;
pub type HSteamUser = c_int;

pub type CreateInterfaceFn = unsafe extern "C" fn(*const c_char, *mut c_int) -> *mut ISteamClient;
pub type SteamGetCallbackFn = unsafe extern "C" fn(HSteamPipe, *mut SteamCallbackMessage, *mut c_int) -> bool;
pub type SteamFreeLastCallbackFn = unsafe extern "C" fn(*const HSteamPipe) -> c_void;

#[repr(C)]
pub struct SteamCallbackMessage {
    pub user: HSteamPipe,
    pub id: c_int,
    pub param_ptr: *mut c_int,
    pub param_size: c_int,
}

// SteamID representation (simplified)
#[repr(C)]
pub struct CSteamID {
    pub m_steamid: u64,
}

#[derive(Debug)]
pub enum SteamError {
    NullVtable,
    PipeCreationFailed,
    PipeReleaseFailed,
    UserConnectionFailed,
    InterfaceCreationFailed(String),
    AppNotFound,
    UnknownError,
}

impl std::fmt::Display for SteamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamError::NullVtable => write!(f, "Steam client vtable is null"),
            SteamError::PipeCreationFailed => write!(f, "Failed to create steam pipe"),
            SteamError::PipeReleaseFailed => write!(f, "Failed to release steam pipe"),
            SteamError::UserConnectionFailed => write!(f, "Failed to connect to steam server"),
            SteamError::InterfaceCreationFailed(name) => write!(f, "Failed to create steam interface: {}", name),
            SteamError::AppNotFound => write!(f, "App not found"),
            SteamError::UnknownError => write!(f, "Unknown Steam error"),
        }
    }
}

impl std::error::Error for SteamError {}