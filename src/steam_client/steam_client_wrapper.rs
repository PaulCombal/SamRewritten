use std::os::raw::{c_char};
use std::sync::Arc;
use libloading::Symbol;
use crate::steam_client::steam_apps_001_vtable::{ISteamApps001, STEAMAPPS001_INTERFACE_VERSION};
use crate::steam_client::steam_apps_001_wrapper::SteamApps001;
use crate::steam_client::steam_apps_vtable::STEAMAPPS_INTERFACE_VERSION;
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steam_client_vtable::{ISteamClient};
use crate::steam_client::steam_user_stats_vtable::STEAMUSERSTATS_INTERFACE_VERSION;
use crate::steam_client::steam_user_stats_wrapper::SteamUserStats;
use crate::steam_client::steam_utils_vtable::STEAMUTILS_INTERFACE_VERSION;
use crate::steam_client::steam_utils_wrapper::SteamUtils;
use crate::steam_client::steamworks_types::{HSteamPipe, HSteamUser, SteamFreeLastCallbackFn, SteamGetCallbackFn};
use crate::steam_client::wrapper_types::SteamError;

pub struct SteamClient {
    inner: Arc<SteamClientInner>,
    // callback_fn: Symbol<'a, SteamGetCallbackFn>,
    // free_callback_fn: Symbol<'a, SteamFreeLastCallbackFn>,
    // running_callback: bool
}

struct SteamClientInner {
    ptr: *mut ISteamClient,
}

impl<'a> SteamClient {
    pub unsafe fn from_raw(ptr: *mut ISteamClient, _callback_fn: Symbol<'a, SteamGetCallbackFn>, _free_callback_fn: Symbol<'a, SteamFreeLastCallbackFn>) -> Self {
        Self {
            inner: Arc::new(SteamClientInner { ptr }),
            // callback_fn,
            // free_callback_fn,
            // running_callback: false
        }
    }

    // pub fn run_callbacks(&mut self, pipe: &HSteamPipe) -> Result<(), SteamError> {
    //     if self.running_callback {
    //         dev_println!("SteamClient running_callback already called");
    //         return Ok(());
    //     }
    //
    //     dev_println!("Steam callbacking.. pipe {}", *pipe);
    //
    //     unsafe {
    //         self.running_callback = true;
    //         let mut message = std::mem::MaybeUninit::<SteamCallbackMessage>::uninit();
    //         let mut call: c_int = -1;
    //         let success = (&self.callback_fn)(*pipe, message.as_mut_ptr(), &mut call);
    //         dev_println!("Steam has callbacks to be taken care of: {}", success);
    //
    //         if success {
    //             let message = message.assume_init();
    //             dev_println!("Callbacked: {message:?}");
    //             // dev_println!("Callback call: {}", call);
    //             let freed = (&self.free_callback_fn)(*pipe);
    //             dev_println!("Callback freed: {}", freed);
    //
    //             if message.id == 1110 {
    //                 dev_println!("received global achievement percentages");
    //                 return Err(SteamError::AppNotFound);
    //             }
    //         }
    //     }
    //
    //     self.running_callback = false;
    //     dev_println!("Steam callback done");
    //     Ok(())
    // }
    
    pub fn create_steam_pipe(&self) -> Result<HSteamPipe, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let pipe = (vtable.create_steam_pipe)(self.inner.ptr);
            if pipe == 0 {
                Err(SteamError::PipeCreationFailed)
            } else {
                Ok(pipe)
            }
        }
    }
    
    pub fn release_steam_pipe(&self, pipe: HSteamPipe) -> Result<bool, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let success = (vtable.release_steam_pipe)(self.inner.ptr, pipe);
            if success {
                Ok(success)
            } else {
                Err(SteamError::PipeReleaseFailed)
            }
        }
    }
    
    pub fn release_user(&self, pipe: HSteamPipe, user: HSteamUser) {
        unsafe {
            let vtable = (*self.inner.ptr).vtable
                .as_ref()
                .expect("SteamClient vtable was null");
            (vtable.release_user)(self.inner.ptr, pipe, user);
        }
    }

    pub fn connect_to_global_user(&self, pipe: HSteamPipe) -> Result<HSteamUser, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let user = (vtable.connect_to_global_user)(self.inner.ptr, pipe);
            if user == 0 {
                Err(SteamError::UserConnectionFailed)
            } else {
                Ok(user)
            }
        }
    }

    pub fn shutdown_if_app_pipes_closed(&self) -> Result<bool, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            Ok((vtable.bshutdown_if_all_pipes_closed)(self.inner.ptr))
        }
    }
    
    pub fn get_isteam_apps(
        &self,
        user: HSteamUser,
        pipe: HSteamPipe,
    ) -> Result<SteamApps, SteamError> {
        unsafe {
            let version = STEAMAPPS_INTERFACE_VERSION.as_ptr() as *const c_char;

            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let apps_ptr = (vtable.get_isteam_apps)(self.inner.ptr, user, pipe, version);

            if apps_ptr.is_null() {
                Err(SteamError::InterfaceCreationFailed("ISteamApps".to_owned()))
            } else {
                Ok(SteamApps::from_raw(apps_ptr))
            }
        }
    }

    pub fn get_isteam_apps_001(
        &self,
        user: HSteamUser,
        pipe: HSteamPipe,
    ) -> Result<SteamApps001, SteamError> {
        unsafe {
            let version = STEAMAPPS001_INTERFACE_VERSION.as_ptr() as *const c_char;

            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let apps_ptr = (vtable.get_isteam_apps)(self.inner.ptr, user, pipe, version);

            if apps_ptr.is_null() {
                Err(SteamError::InterfaceCreationFailed("ISteamApps001".to_owned()))
            } else {
                Ok(SteamApps001::from_raw(apps_ptr as *mut ISteamApps001))
            }
        }
    }

    pub fn get_isteam_utils(
        &self,
        pipe: HSteamPipe,
    ) -> Result<SteamUtils, SteamError> {
        unsafe {
            let version = STEAMUTILS_INTERFACE_VERSION.as_ptr() as *const c_char;

            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let utils_ptr = (vtable.get_isteam_utils)(self.inner.ptr, pipe, version);

            if utils_ptr.is_null() {
                Err(SteamError::InterfaceCreationFailed("ISteamUtils".to_owned()))
            } else {
                Ok(SteamUtils::from_raw(utils_ptr))
            }
        }
    }

    pub fn get_isteam_user_stats(
        &self,
        user: HSteamUser,
        pipe: HSteamPipe,
    ) -> Result<SteamUserStats, SteamError> {
        unsafe {
            let version = STEAMUSERSTATS_INTERFACE_VERSION.as_ptr() as *const c_char;

            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            let user_stats_ptr = (vtable.get_isteam_user_stats)(self.inner.ptr, user, pipe, version);

            if user_stats_ptr.is_null() {
                Err(SteamError::InterfaceCreationFailed("ISteamUtils".to_owned()))
            } else {
                Ok(SteamUserStats::from_raw(user_stats_ptr))
            }
        }
    }
}