use std::os::raw::{c_int, c_char, c_void};
use crate::steam_client::types::{CSteamID, SteamAPICall_t};

#[repr(C)]
pub enum ESteamAPICallFailure {
    None = -1,
    SteamGone = 0,
    NetworkFailure = 1,
    InvalidHandle = 2,
    MismatchedCallback = 3,
}

#[repr(C)]
pub enum EGamepadTextInputMode {
    Normal = 0,
    Password = 1,
}

#[repr(C)]
pub enum EGamepadTextInputLineMode {
    SingleLine = 0,
    MultipleLines = 1,
}

#[repr(C)]
pub enum EFloatingGamepadTextInputMode {
    ModeSingleLine = 0,
    ModeMultipleLines = 1,
    ModeEmail = 2,
    ModeNumeric = 3,
}

#[repr(C)]
pub enum ETextFilteringContext {
    Unknown = 0,
    GameContent = 1,
    Chat = 2,
    Name = 3,
}

#[repr(C)]
pub enum EUniverse {
    Invalid = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    Max = 5,
}

#[repr(C)]
pub enum ENotificationPosition {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

#[repr(C)]
pub enum ESteamIPv6ConnectivityState {
    Unknown = 0,
    Good = 1,
    Bad = 2,
}

#[repr(C)]
pub enum ESteamIPv6ConnectivityProtocol {
    HTTP = 0,
    UDP = 1,
}

#[repr(C)]
pub enum ECheckFileSignature {
    InvalidSignature = 0,
    ValidSignature = 1,
    FileNotFound = 2,
    NoSignaturesFoundForThisApp = 3,
    NoSignaturesFoundForThisFile = 4,
}

// Callback type for warning messages
pub type SteamAPIWarningMessageHook_t = unsafe extern "C" fn(c_int, *const c_char);

#[repr(C)]
pub struct ISteamUtilsVTable {
    pub get_seconds_since_app_active: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub get_seconds_since_computer_active: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub get_connected_universe: unsafe extern "C" fn(*mut ISteamUtils) -> EUniverse,
    pub get_server_real_time: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub get_ip_country: unsafe extern "C" fn(*mut ISteamUtils) -> *const c_char,
    pub get_image_size: unsafe extern "C" fn(*mut ISteamUtils, c_int, *mut u32, *mut u32) -> bool,
    pub get_image_rgba: unsafe extern "C" fn(*mut ISteamUtils, c_int, *mut c_char, c_int) -> bool,
    pub get_cserip_port: unsafe extern "C" fn(*mut ISteamUtils, *mut u32, *mut u16) -> bool, // Deprecated
    pub get_current_battery_power: unsafe extern "C" fn(*mut ISteamUtils) -> u8,
    pub get_app_id: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub set_overlay_notification_position: unsafe extern "C" fn(*mut ISteamUtils, ENotificationPosition),
    pub is_api_call_completed: unsafe extern "C" fn(*mut ISteamUtils, SteamAPICall_t, *mut bool) -> bool,
    pub get_api_call_failure_reason: unsafe extern "C" fn(*mut ISteamUtils, SteamAPICall_t) -> ESteamAPICallFailure,
    pub get_api_call_result: unsafe extern "C" fn(
        *mut ISteamUtils,
        SteamAPICall_t,
        *mut c_void,
        c_int,
        c_int,
        *mut bool
    ) -> bool,
    pub run_frame: unsafe extern "C" fn(*mut ISteamUtils), // Deprecated
    pub get_ipc_call_count: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub set_warning_message_hook: unsafe extern "C" fn(*mut ISteamUtils, SteamAPIWarningMessageHook_t),
    pub is_overlay_enabled: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub b_overlay_needs_present: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub check_file_signature: unsafe extern "C" fn(*mut ISteamUtils, *const c_char) -> SteamAPICall_t,
    pub show_gamepad_text_input: unsafe extern "C" fn(
        *mut ISteamUtils,
        EGamepadTextInputMode,
        EGamepadTextInputLineMode,
        *const c_char,
        u32,
        *const c_char
    ) -> bool,
    pub get_entered_gamepad_text_length: unsafe extern "C" fn(*mut ISteamUtils) -> u32,
    pub get_entered_gamepad_text_input: unsafe extern "C" fn(*mut ISteamUtils, *mut c_char, u32) -> bool,
    pub get_steam_ui_language: unsafe extern "C" fn(*mut ISteamUtils) -> *const c_char,
    pub is_steam_running_in_vr: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub set_overlay_notification_inset: unsafe extern "C" fn(*mut ISteamUtils, c_int, c_int),
    pub is_steam_in_big_picture_mode: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub start_vr_dashboard: unsafe extern "C" fn(*mut ISteamUtils),
    pub is_vr_headset_streaming_enabled: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub set_vr_headset_streaming_enabled: unsafe extern "C" fn(*mut ISteamUtils, bool),
    pub is_steam_china_launcher: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub init_filter_text: unsafe extern "C" fn(*mut ISteamUtils, u32) -> bool,
    pub filter_text: unsafe extern "C" fn(
        *mut ISteamUtils,
        ETextFilteringContext,
        CSteamID,
        *const c_char,
        *mut c_char,
        u32
    ) -> c_int,
    pub get_ipv6_connectivity_state: unsafe extern "C" fn(
        *mut ISteamUtils,
        ESteamIPv6ConnectivityProtocol
    ) -> ESteamIPv6ConnectivityState,
    pub is_steam_running_on_steam_deck: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub show_floating_gamepad_text_input: unsafe extern "C" fn(
        *mut ISteamUtils,
        EFloatingGamepadTextInputMode,
        c_int,
        c_int,
        c_int,
        c_int
    ) -> bool,
    pub set_game_launcher_mode: unsafe extern "C" fn(*mut ISteamUtils, bool),
    pub dismiss_floating_gamepad_text_input: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
    pub dismiss_gamepad_text_input: unsafe extern "C" fn(*mut ISteamUtils) -> bool,
}

#[repr(C)]
pub struct ISteamUtils {
    pub vtable: *const ISteamUtilsVTable,
}

pub const STEAMUTILS_INTERFACE_VERSION: &str = "SteamUtils010\0";