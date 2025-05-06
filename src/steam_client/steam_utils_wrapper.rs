#![allow(dead_code)]

use std::sync::Arc;
use crate::steam_client::steam_utils_vtable::ISteamUtils;
use crate::steam_client::steamworks_types::{AppId_t};
use crate::steam_client::wrapper_error::SteamError;

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
            let vtable = (*self.inner.ptr).vtable.as_ref()
                .ok_or(SteamError::NullVtable)?;
            let app_id = (vtable.get_app_id)(self.inner.ptr);
            
            Ok(app_id)
        }
    }
}