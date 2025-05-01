use std::os::raw::{c_int, c_char, c_void};
use crate::steam_client::steam_app_list_vtable::ISteamAppList;
use crate::steam_client::steam_apps_vtable::ISteamApps;
use crate::steam_client::steam_user_stats_vtable::ISteamUserStats;
use crate::steam_client::steam_utils_vtable::ISteamUtils;
use crate::steam_client::types::{HSteamPipe, HSteamUser};

// Define SteamIPAddress_t (simplified - actual implementation may vary)
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct SteamIPAddress_t {
    // Implementation depends on actual definition
    _unused: [u8; 0],
}

// Define callback types
#[allow(non_camel_case_types)]

pub type SteamAPIWarningMessageHook_t = extern "C" fn(c_int, *const c_char);

#[allow(non_camel_case_types)]

pub type SteamAPI_CheckCallbackRegistered_t = extern "C" fn();

// Define account type enum
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum EAccountType {
    k_EAccountTypeInvalid = 0,
    k_EAccountTypeIndividual = 1,
    // ... other account types
}

// Forward declarations for other interfaces
#[repr(C)]
pub struct ISteamUser;
#[repr(C)]
pub struct ISteamGameServer;
// ... other interface forward declarations

// The complete vtable structure
#[repr(C)]
pub struct ISteamClientVTable {
    pub create_steam_pipe: unsafe extern "C" fn(*mut ISteamClient) -> HSteamPipe,
    pub release_steam_pipe: unsafe extern "C" fn(*mut ISteamClient, HSteamPipe) -> bool,
    pub connect_to_global_user: unsafe extern "C" fn(*mut ISteamClient, HSteamPipe) -> HSteamUser,
    pub create_local_user: unsafe extern "C" fn(*mut ISteamClient, *mut HSteamPipe, EAccountType) -> HSteamUser,
    pub release_user: unsafe extern "C" fn(*mut ISteamClient, HSteamPipe, HSteamUser),
    pub get_isteam_user: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut ISteamUser,
    pub get_isteam_game_server: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut ISteamGameServer,
    pub set_local_ip_binding: unsafe extern "C" fn(*mut ISteamClient, *const SteamIPAddress_t, u16),
    pub get_isteam_friends: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamFriends
    pub get_isteam_utils: unsafe extern "C" fn(*mut ISteamClient, HSteamPipe, *const c_char) -> *mut ISteamUtils, // ISteamUtils
    pub get_isteam_matchmaking: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamMatchmaking
    pub get_isteam_matchmaking_servers: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamMatchmakingServers
    pub get_isteam_generic_interface: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void,
    pub get_isteam_user_stats: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut ISteamUserStats, // ISteamUserStats
    pub get_isteam_game_server_stats: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamGameServerStats
    pub get_isteam_apps: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut ISteamApps, // ISteamApps
    pub get_isteam_networking: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamNetworking
    pub get_isteam_remote_storage: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamRemoteStorage
    pub get_isteam_screenshots: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamScreenshots
    pub get_isteam_game_search: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamGameSearch
    pub run_frame: unsafe extern "C" fn(*mut ISteamClient),
    pub get_ipc_call_count: unsafe extern "C" fn(*mut ISteamClient) -> u32,
    pub set_warning_message_hook: unsafe extern "C" fn(*mut ISteamClient, SteamAPIWarningMessageHook_t),
    pub bshutdown_if_all_pipes_closed: unsafe extern "C" fn(*mut ISteamClient) -> bool,
    pub get_isteam_http: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamHTTP
    pub deprecated_get_isteam_unified_messages: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void,
    pub get_isteam_controller: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamController
    pub get_isteam_ugc: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamUGC
    pub get_isteam_app_list: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut ISteamAppList, // ISteamAppList
    pub get_isteam_music: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamMusic
    pub get_isteam_music_remote: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamMusicRemote
    pub get_isteam_html_surface: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamHTMLSurface
    pub deprecated_set_steam_api_cpost_api_result_in_process: unsafe extern "C" fn(*mut ISteamClient, Option<extern "C" fn()>),
    pub deprecated_remove_steam_api_cpost_api_result_in_process: unsafe extern "C" fn(*mut ISteamClient, Option<extern "C" fn()>),
    pub set_steam_api_ccheck_callback_registered_in_process: unsafe extern "C" fn(*mut ISteamClient, SteamAPI_CheckCallbackRegistered_t),
    pub get_isteam_inventory: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamInventory
    pub get_isteam_video: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamVideo
    pub get_isteam_parental_settings: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamParentalSettings
    pub get_isteam_input: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamInput
    pub get_isteam_parties: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamParties
    pub get_isteam_remote_play: unsafe extern "C" fn(*mut ISteamClient, HSteamUser, HSteamPipe, *const c_char) -> *mut c_void, // ISteamRemotePlay
    pub destroy_all_interfaces: unsafe extern "C" fn(*mut ISteamClient),
}

// The main interface structure
#[repr(C)]
pub struct ISteamClient {
    pub vtable: *const ISteamClientVTable,
}

// Interface version constant
pub const STEAMCLIENT_INTERFACE_VERSION: &str = "SteamClient020\0";
