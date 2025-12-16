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

use crate::backend::stat_definitions::StatInfo;
use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct GStatObject(ObjectSubclass<imp::GStatObject>);
}

impl GStatObject {
    pub fn new(info: StatInfo) -> Self {
        match info {
            StatInfo::Float(info) => Object::builder()
                .property("id", info.id)
                .property("app-id", info.app_id)
                .property("display-name", info.display_name)
                .property("original-value", info.original_value as f64)
                .property("current-value", info.float_value as f64)
                .property("is-increment-only", info.is_increment_only)
                .property("permission", info.permission)
                .property("is-integer", false)
                .build(),
            StatInfo::Integer(info) => Object::builder()
                .property("id", info.id)
                .property("app-id", info.app_id)
                .property("display-name", info.display_name)
                .property("original-value", info.original_value as f64)
                .property("current-value", info.int_value as f64)
                .property("is-increment-only", info.is_increment_only)
                .property("permission", info.permission)
                .property("is-integer", true)
                .build(),
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
    #[properties(wrapper_type = super::GStatObject)]
    pub struct GStatObject {
        #[property(get, set)]
        id: RefCell<String>,

        #[property(get, set)]
        display_name: RefCell<String>,

        #[property(get, set)]
        original_value: Cell<f64>,

        #[property(get, set)]
        current_value: Cell<f64>,

        #[property(get, set)]
        is_integer: Cell<bool>,

        #[property(get, set)]
        is_increment_only: Cell<bool>,

        #[property(get, set)]
        app_id: Cell<u32>,

        #[property(get, set)]
        permission: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GStatObject {
        const NAME: &'static str = "GStatObject";
        type Type = super::GStatObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GStatObject {}
}
