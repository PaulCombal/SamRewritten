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

// CClientUserMap vtable has ~260 slots and Steam inserts methods into it: the
// October 2026 beta moved GetSubscribedApps from 182 to 183. The slot is
// therefore looked up by name at runtime, with 182 as the fallback.

use crate::steam_client::steamworks_types::AppId_t;
use std::os::raw::c_void;

#[repr(C)]
pub struct IClientUser {
    pub vtable: *const *const c_void,
}

pub const GET_SUBSCRIBED_APPS_SLOT: usize = 182;
pub const GET_SUBSCRIBED_APPS_NAME: &str = "GetSubscribedApps";

/// The bool adds expired and just-granted licences: `true` answers a
/// superset, it is not an ownership filter.
pub type GetSubscribedAppsFn =
    unsafe extern "C" fn(*mut IClientUser, *mut AppId_t, u32, bool) -> u32;
