// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::dev_println;
use crate::steam_client::client_engine_wrapper::ClientEngineInner;
use crate::steam_client::client_user_vtable::{
    GET_SUBSCRIBED_APPS_NAME, GET_SUBSCRIBED_APPS_SLOT, GetSubscribedAppsFn, IClientUser,
};
use crate::steam_client::steamworks_types::AppId_t;
use crate::steam_client::vtable_probe::find_slot_by_name;
use std::os::raw::c_void;
use std::rc::Rc;

const MAX_SUBSCRIBED_APPS: u32 = 1 << 20;
const SLOT_SEARCH_RADIUS: usize = 16;

pub struct ClientUser {
    inner: Rc<ClientUserInner>,
}

struct ClientUserInner {
    ptr: *mut IClientUser,
    get_subscribed_apps: GetSubscribedAppsFn,
    _engine: Rc<ClientEngineInner>,
}

impl ClientUser {
    pub unsafe fn from_raw(ptr: *mut IClientUser, engine: Rc<ClientEngineInner>) -> Self {
        let vtable = unsafe { (*ptr).vtable };
        let slot = unsafe {
            find_slot_by_name(
                vtable,
                GET_SUBSCRIBED_APPS_SLOT,
                SLOT_SEARCH_RADIUS,
                GET_SUBSCRIBED_APPS_NAME,
            )
        }
        .unwrap_or_else(|| {
            dev_println!(
                "CLIENT USER",
                "GetSubscribedApps not found by name, assuming slot {GET_SUBSCRIBED_APPS_SLOT}"
            );
            GET_SUBSCRIBED_APPS_SLOT
        });
        dev_println!("CLIENT USER", "GetSubscribedApps at slot {slot}");
        let get_subscribed_apps =
            unsafe { std::mem::transmute::<*const c_void, GetSubscribedAppsFn>(*vtable.add(slot)) };
        Self {
            inner: Rc::new(ClientUserInner {
                ptr,
                get_subscribed_apps,
                _engine: engine,
            }),
        }
    }

    /// Returns the full list of app ids the user is subscribed to (owns,
    /// rents, has via family sharing, …). One IPC roundtrip for the count,
    /// one for the data — irrespective of library size.
    pub fn get_subscribed_apps(&self) -> Vec<AppId_t> {
        unsafe {
            let get_subscribed_apps = self.inner.get_subscribed_apps;
            let count = get_subscribed_apps(self.inner.ptr, std::ptr::null_mut(), 0, false);
            if count > MAX_SUBSCRIBED_APPS {
                return Vec::new();
            }
            let mut buf: Vec<AppId_t> = vec![0; count as usize];
            let written = get_subscribed_apps(self.inner.ptr, buf.as_mut_ptr(), count, false);
            // Handed too small a buffer, Steam writes nothing and still
            // reports the count, so the zeroes are not app ids.
            buf.truncate(if written > count { 0 } else { written as usize });
            buf
        }
    }
}
