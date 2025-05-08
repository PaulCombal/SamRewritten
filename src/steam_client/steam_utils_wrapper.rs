#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::sync::Arc;
use crate::dev_println;
use crate::steam_client::steam_utils_vtable::ISteamUtils;
use crate::steam_client::steamworks_types::{AppId_t, GlobalAchievementPercentagesReady_t, SteamAPICall_t};
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
    pub fn is_api_call_completed(&self, api_call_handle: SteamAPICall_t) -> Result<bool, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref()
                .ok_or(SteamError::NullVtable)?;
            let mut b_failed = true;
            let completed = (vtable.is_api_call_completed)(self.inner.ptr, api_call_handle, &mut b_failed);
            
            if b_failed {
                dev_println!("is_api_call_completed failed");
                return Err(SteamError::UnknownError);
            }

            Ok(completed)
        }
    }
    
    pub fn get_api_call_result<T>(&self, api_call_handle: SteamAPICall_t) -> Result<T, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;
            
            let mut b_failed = true;
            let mut result: T = std::mem::zeroed();
            let success = (vtable.get_api_call_result)(
                self.inner.ptr,
                api_call_handle,
                &mut result as *mut T as *mut c_void,
                size_of::<GlobalAchievementPercentagesReady_t>() as c_int,
                1110,
                &mut b_failed,
            );

            if b_failed {
                dev_println!("get_api_call_result failed");
                return Err(SteamError::UnknownError);
            }
            
            if !success {
                dev_println!("get_api_call_result not success");
                return Err(SteamError::UnknownError);
            }

            Ok(result)
        }
    }
}