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

use glib::Object;
use gtk::glib;

pub const MODE_AUTOCOMMIT: &str = "autocommit";
pub const MODE_DEFERRED: &str = "deferred";
pub const MODE_COPY_TIMING: &str = "copytiming";

glib::wrapper! {
    pub struct GUnlockModeState(ObjectSubclass<imp::GUnlockModeState>);
}

impl Default for GUnlockModeState {
    fn default() -> Self {
        Object::builder().property("mode", MODE_AUTOCOMMIT).build()
    }
}

mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GUnlockModeState)]
    pub struct GUnlockModeState {
        #[property(get, set)]
        mode: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GUnlockModeState {
        const NAME: &'static str = "GUnlockModeState";
        type Type = super::GUnlockModeState;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GUnlockModeState {}
}
