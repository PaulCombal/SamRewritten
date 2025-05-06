use std::sync::Arc;
use crate::steam_client::steam_apps_vtable::ISteamApps;
use crate::steam_client::wrapper_error::SteamError;

pub struct SteamApps {
    inner: Arc<SteamAppsInner>,
}

struct SteamAppsInner {
    ptr: *mut ISteamApps,
}

impl SteamApps {
    pub unsafe fn from_raw(ptr: *mut ISteamApps) -> Self {
        Self {
            inner: Arc::new(SteamAppsInner { ptr }),
        }
    }
    
    pub fn get_current_game_language(&self) -> String {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().expect("Null ISteamApps vtable");
            let lang_ptr = (vtable.get_current_game_language)(self.inner.ptr);
            std::ffi::CStr::from_ptr(lang_ptr)
                .to_string_lossy()
                .into_owned()
        }
    }
    
    pub fn is_subscribed_app(&self, app_id: u32) -> Result<bool, SteamError> {
        unsafe {
            // Get the vtable - return error if null
            let vtable = (*self.inner.ptr).vtable.as_ref()
                .ok_or(SteamError::NullVtable)?;

            // Call through the vtable
            let is_subscribed = (vtable.b_is_subscribed_app)(self.inner.ptr, app_id);

            Ok(is_subscribed)
        }
    }
}
