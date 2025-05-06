use crate::steam_client::steam_client_vtable::{ISteamClient, STEAMCLIENT_INTERFACE_VERSION};
use crate::steam_client::steam_client_wrapper::SteamClient;
use crate::steam_client::steamworks_types::{CreateInterfaceFn, SteamFreeLastCallbackFn, SteamGetCallbackFn};
use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

static STEAM_CLIENT_LIB: OnceLock<Library> = OnceLock::new(); // Make the lifetime 'static

#[cfg(target_os = "linux")]
pub fn load_steamclient_library() -> Result<Library, Box<dyn std::error::Error>> {
    unsafe {
        let home = std::env::var("HOME")?;
        let lib_steamclient_path = PathBuf::from(home + "/snap/steam/common/.local/share/Steam/linux64/steamclient.so");
        let lib_steamclient = Library::new(lib_steamclient_path)?;
        Ok(lib_steamclient)
    }
}

#[cfg(target_os = "windows")]
pub fn load_steamclient_library() -> Result<Library, Box<dyn std::error::Error>> {
    use libloading::os::windows::{self, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS};

    unsafe {
        let program_files = std::env::var("ProgramFiles(x86)")?;
        #[cfg(target_pointer_width = "64")]
        let lib_steamclient_path = PathBuf::from(program_files + "\\Steam\\steamclient64.dll");
        #[cfg(target_pointer_width = "32")]
        let lib_steamclient_path = PathBuf::from(program_files + "\\Steam\\steamclient.dll");
        Ok(windows::Library::load_with_flags(
            lib_steamclient_path,
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
        )?.into())
    }
}

pub fn new_steam_client_interface(
    steamclient_so: &Library,
) -> Result<
    (
        *mut ISteamClient,
        Symbol<SteamGetCallbackFn>,
        Symbol<SteamFreeLastCallbackFn>,
    ),
    Box<dyn std::error::Error>,
> {
    unsafe {
        let create_interface: Symbol<CreateInterfaceFn> = steamclient_so.get(b"CreateInterface")?;
        let steam_get_callback: Symbol<SteamGetCallbackFn> = steamclient_so.get(b"Steam_BGetCallback")?;
        let steam_free_last_callback: Symbol<SteamFreeLastCallbackFn> = steamclient_so.get(b"Steam_FreeLastCallback")?; 

        let mut return_code = 1;
        let client = create_interface(
            STEAMCLIENT_INTERFACE_VERSION.as_ptr() as *const c_char,
            &mut return_code,
        );

        if return_code != 0 {
            return Err(Box::from(format!(
                "Steam client interface creation failed with code {}",
                return_code
            )));
        }

        if client.is_null() {
            return Err(Box::from(
                "Steam client failed to create interface (pointer is NULL)",
            ));
        }

        Ok((client, steam_get_callback, steam_free_last_callback))
    }
}

pub fn create_steam_client() -> Result<SteamClient<'static>, Box<dyn std::error::Error>> {
    if STEAM_CLIENT_LIB.get().is_none() {
        let steamclient_so = load_steamclient_library()?;
        match STEAM_CLIENT_LIB.set(steamclient_so) {
            Ok(_) => {}
            Err(_) => panic!("Failed to create steam client"),
        };
    }

    let (raw_client, callback_fn, free_callback_fn) = new_steam_client_interface(&STEAM_CLIENT_LIB.get().unwrap())?;
    let client = unsafe { SteamClient::from_raw(raw_client, callback_fn, free_callback_fn) };

    Ok(client)
}