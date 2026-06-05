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

use crate::backend::user_unlock_times::Friend;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct GFriendObject(ObjectSubclass<imp::GFriendObject>);
}

impl GFriendObject {
    pub fn new(friend: &Friend) -> Self {
        Object::builder()
            .property("name", &friend.name)
            .property("steam-id", friend.steam_id64)
            .property("avatar-url", &friend.avatar_url)
            .property(
                "search-text",
                format!("{} {}", friend.name, friend.steam_id64).to_lowercase(),
            )
            .property("is-custom", false)
            .build()
    }

    /// The placeholder row that selects a pasted SteamID64; its `steam-id` is
    /// updated from the search text as the user types.
    pub fn custom() -> Self {
        Object::builder().property("is-custom", true).build()
    }

    pub fn to_friend(&self) -> Friend {
        Friend {
            name: self.name(),
            steam_id64: self.steam_id(),
            avatar_url: self.avatar_url(),
        }
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GFriendObject)]
    pub struct GFriendObject {
        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set)]
        steam_id: Cell<u64>,

        #[property(get, set)]
        avatar_url: RefCell<String>,

        #[property(get, set)]
        search_text: RefCell<String>,

        #[property(get, set)]
        is_custom: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GFriendObject {
        const NAME: &'static str = "GFriendObject";
        type Type = super::GFriendObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GFriendObject {}
}
