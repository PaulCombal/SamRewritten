use std::sync::Arc;
use crate::steam_client::steam_utils_vtable::ISteamUtils;
use crate::steam_client::types::{AppId_t, SteamError};

pub struct SteamUtils {
    inner: Arc<SteamUtilsInner>,
}

struct SteamUtilsInner {
    ptr: *mut ISteamUtils,
}

impl SteamUtils {
    pub unsafe fn from_raw(ptr: *mut ISteamUtils) -> Self {
        Self {
            inner: Arc::new(SteamUtilsInner { ptr }),
        }
    }
    
    pub fn get_app_id(&self) -> Result<AppId_t, SteamError> {
        unsafe {
            // Get the vtable - return error if null
            let vtable = (*self.inner.ptr).vtable.as_ref()
                .ok_or(SteamError::NullVtable)?;

            // Call through the vtable
            let app_id = (vtable.get_app_id)(self.inner.ptr);

            Ok(app_id)
        }
    }
}