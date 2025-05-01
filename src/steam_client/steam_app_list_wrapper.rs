use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use crate::steam_client::steam_app_list_vtable::ISteamAppList;
use crate::steam_client::types::{AppId_t, SteamError};

pub struct SteamAppList {
    inner: Arc<SteamAppListInner>,
}

struct SteamAppListInner {
    ptr: *mut ISteamAppList,
}

impl SteamAppList {
    pub unsafe fn from_raw(ptr: *mut ISteamAppList) -> Self {
        Self {
            inner: Arc::new(SteamAppListInner { ptr }),
        }
    }

    pub fn get_app_name(&self, app_id: AppId_t) -> Result<String, SteamError> {
        let mut buffer = vec![0u8; 256];

        unsafe {
            let vtable = (*self.inner.ptr).vtable.as_ref().ok_or(SteamError::NullVtable)?;

            let result = (vtable.get_app_name)(
                self.inner.ptr,
                app_id,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as c_int
            );

            match result {
                -1 => Err(SteamError::AppNotFound),
                len if len >= 0 => {
                    // Convert the null-terminated C string to a Rust string
                    let c_str = CStr::from_ptr(buffer.as_ptr() as *const c_char);
                    Ok(c_str.to_string_lossy().into_owned())
                },
                _ => Err(SteamError::UnknownError),
            }
        }
    }
}