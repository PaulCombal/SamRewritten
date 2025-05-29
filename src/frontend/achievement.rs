// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
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

use glib::Object;
use gtk::glib;

use crate::backend::stat_definitions::AchievementInfo;

glib::wrapper! {
    pub struct GAchievementObject(ObjectSubclass<imp::GAchievementObject>);
}

impl GAchievementObject {
    pub fn new(info: AchievementInfo) -> Self {
        let global_achieved_percent = info.global_achieved_percent.unwrap_or(0.0);
        let global_achieved_percent_ok = info.global_achieved_percent.is_some();

        Object::builder()
            .property("search-text", format!("{} {}", info.name, info.description))
            .property("id", info.id)
            .property("name", info.name)
            .property("description", info.description)
            .property("is-achieved", info.is_achieved)
            .property(
                "unlock-time",
                info.unlock_time.map(|time| format!("{time:#?}")),
            )
            .property("icon-normal", info.icon_normal)
            .property("icon-locked", info.icon_locked)
            .property("permission", info.permission)
            .property("global-achieved-percent", global_achieved_percent)
            .property("global-achieved-percent-ok", global_achieved_percent_ok)
            .build()
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GAchievementObject)]
    pub struct GAchievementObject {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        name: RefCell<String>,

        #[property(get, set)]
        description: RefCell<String>,

        #[property(get, set)]
        search_text: RefCell<String>,

        #[property(get, set)]
        is_achieved: Cell<bool>,

        #[property(get, set)]
        unlock_time: RefCell<Option<String>>,

        #[property(get, set)]
        icon_normal: RefCell<String>,

        #[property(get, set)]
        icon_locked: RefCell<String>,

        #[property(get, set)]
        permission: Cell<i32>,

        #[property(get, set)]
        global_achieved_percent: Cell<f32>,

        #[property(get, set)]
        global_achieved_percent_ok: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GAchievementObject {
        const NAME: &'static str = "GAchievementObject";
        type Type = super::GAchievementObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GAchievementObject {}
}
