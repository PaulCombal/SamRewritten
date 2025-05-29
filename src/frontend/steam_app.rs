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

use crate::backend::app_lister::AppModel;
use glib::Object;
use gtk::glib;

// ANCHOR: integer_object
glib::wrapper! {
    pub struct GSteamAppObject(ObjectSubclass<imp::GSteamAppObject>);
}

impl GSteamAppObject {
    pub fn new(app: AppModel) -> Self {
        Object::builder()
            .property("app_id", app.app_id)
            .property("app_name", app.app_name)
            .property("developer", app.developer) 
            .property("image_url", app.image_url)
            .property("metacritic_score", app.metacritic_score.unwrap_or(u8::MAX))
            .property("app_type", format!("{:?}", app.app_type))
            .build()
    }
}
// ANCHOR_END: integer_object

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GSteamAppObject)]
    pub struct GSteamAppObject {
        #[property(get, set)]
        app_id: Cell<u32>,

        #[property(get, set)]
        app_name: RefCell<String>,

        #[property(get, set)]
        developer: RefCell<String>,

        #[property(get, set)]
        metacritic_score: Cell<u8>,

        #[property(get, set)]
        image_url: RefCell<Option<String>>,

        #[property(get, set)]
        app_type: RefCell<String>
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for GSteamAppObject {
        const NAME: &'static str = "GSteamAppObject";
        type Type = super::GSteamAppObject;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for GSteamAppObject {}
}
