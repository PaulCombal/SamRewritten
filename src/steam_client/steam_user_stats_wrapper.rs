use crate::steam_client::steam_user_stats_vtable::ISteamUserStats;
use std::sync::Arc;
use crate::steam_client::steamworks_types::SteamAPICall_t;
use crate::steam_client::wrapper_error::SteamError;

pub struct SteamUserStats {
    inner: Arc<SteamUserStatsInner>,
}

struct SteamUserStatsInner {
    ptr: *mut ISteamUserStats,
}

impl SteamUserStats {
    pub unsafe fn from_raw(ptr: *mut ISteamUserStats) -> Self {
        Self {
            inner: Arc::new(SteamUserStatsInner { ptr }),
        }
    }

    pub fn get_achievement_and_unlock_time(
        &self,
        achievement_name: &str,
    ) -> Result<(bool, u32), SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let mut achieved = false;
            let mut unlock_time = 0u32;
            let c_achievement_name =
                std::ffi::CString::new(achievement_name).map_err(|_| SteamError::UnknownError)?;

            let success = (vtable.get_achievement_and_unlock_time)(
                self.inner.ptr,
                c_achievement_name.as_ptr(),
                &mut achieved,
                &mut unlock_time,
            );

            if success {
                Ok((achieved, unlock_time))
            } else {
                Err(SteamError::UnknownError)
            }
        }
    }

    pub fn set_achievement(
        &self,
        achievement_name: &str,
    ) -> Result<(), SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;
            
            let c_achievement_name =
                std::ffi::CString::new(achievement_name).map_err(|_| SteamError::UnknownError)?;

            let success = (vtable.set_achievement)(
                self.inner.ptr,
                c_achievement_name.as_ptr(),
            );

            if success {
                Ok(())
            } else {
                Err(SteamError::UnknownError)
            }
        }
    }

    pub fn clear_achievement(
        &self,
        achievement_name: &str,
    ) -> Result<(), SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let c_achievement_name =
                std::ffi::CString::new(achievement_name).map_err(|_| SteamError::UnknownError)?;

            let success = (vtable.clear_achievement)(
                self.inner.ptr,
                c_achievement_name.as_ptr(),
            );

            if success {
                Ok(())
            } else {
                Err(SteamError::UnknownError)
            }
        }
    }
    
    pub fn get_stat_i32(&self, stat_name: &str) -> Result<i32, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;
            
            let c_stat_name =
                std::ffi::CString::new(stat_name).map_err(|_| SteamError::UnknownError)?;
            let mut stat_value = 0i32;
            
            let success = (vtable.get_stat_int32)(
                self.inner.ptr,
                c_stat_name.as_ptr(),
                &mut stat_value
            );
            
            if success == false { 
                return Err(SteamError::UnknownError);
            }
            
            Ok(stat_value)
        }
    }

    pub fn get_stat_float(&self, stat_name: &str) -> Result<f32, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let c_stat_name =
                std::ffi::CString::new(stat_name).map_err(|_| SteamError::UnknownError)?;
            let mut stat_value = 0f32;

            let success = (vtable.get_stat_float)(
                self.inner.ptr,
                c_stat_name.as_ptr(),
                &mut stat_value
            );

            if success == false {
                return Err(SteamError::UnknownError);
            }

            Ok(stat_value)
        }
    }

    pub fn set_stat_i32(&self, stat_name: &str, stat_value: i32) -> Result<i32, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let c_stat_name =
                std::ffi::CString::new(stat_name).map_err(|_| SteamError::UnknownError)?;

            let success = (vtable.set_stat_int32)(
                self.inner.ptr,
                c_stat_name.as_ptr(),
                stat_value
            );

            if success == false {
                return Err(SteamError::UnknownError);
            }

            Ok(stat_value)
        }
    }

    pub fn set_stat_float(&self, stat_name: &str, stat_value: f32) -> Result<f32, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let c_stat_name =
                std::ffi::CString::new(stat_name).map_err(|_| SteamError::UnknownError)?;

            let success = (vtable.set_stat_float)(
                self.inner.ptr,
                c_stat_name.as_ptr(),
                stat_value
            );

            if success == false {
                return Err(SteamError::UnknownError);
            }

            Ok(stat_value)
        }
    }

    pub fn request_global_achievement_percentages(&self) -> Result<SteamAPICall_t, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let res = (vtable.request_global_achievement_percentages)(
                self.inner.ptr,
            );

            if res == 0 {
                return Err(SteamError::UnknownError);
            }

            Ok(res)
        }
    }
    
    pub fn get_achievement_achieved_percent(&self, achievement_name: &str) -> Result<f32, SteamError> {
        unsafe {
            let vtable = (*self.inner.ptr)
                .vtable
                .as_ref()
                .ok_or(SteamError::NullVtable)?;

            let c_achievement_name =
                std::ffi::CString::new(achievement_name).map_err(|_| SteamError::UnknownError)?;
            let mut achieved_percent = 0f32;

            let success = (vtable.get_achievement_achieved_percent)(
                self.inner.ptr,
                c_achievement_name.as_ptr(),
                &mut achieved_percent,
            );

            if !success {
                return Err(SteamError::UnknownError);
            }

            Ok(achieved_percent)
        }
    }
}
