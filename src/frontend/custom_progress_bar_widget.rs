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

use gtk::glib;

// This widget is used to display a plain color on a given percentage of width
// use "value" to indicate the percentage of width to fill
// minimum: 0, maximum: 100

glib::wrapper! {
    pub struct CustomProgressBar(ObjectSubclass<imp::CustomProgressBar>)
        @extends gtk::Widget;
}

impl CustomProgressBar {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("value", 0f32) // Initialize value to 0
            .build()
    }
}

mod imp {
    use glib::Properties;
    use gtk::gdk::RGBA;
    use gtk::glib::{self};
    use gtk::graphene::Rect;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::Cell;

    // If building with Adwaita, use the platform accent color
    const BAR_COLOR: RGBA = RGBA::new(0.92, 0.92, 1.0, 1.0);

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::CustomProgressBar)]
    pub struct CustomProgressBar {
        #[property(get, set)]
        pub value: Cell<f32>, // Value from 0 to 100
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CustomProgressBar {
        const NAME: &'static str = "CustomProgressBar";
        type Type = super::CustomProgressBar;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for CustomProgressBar {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_size_request(200, 20); // Set a default size for the progress bar
        }
    }

    impl WidgetImpl for CustomProgressBar {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            let value = self.value.get();

            let progress_width = width * (value / 100.0);

            // Draw the background of the progress bar
            // let background_rect = Rect::new(0.0, 0.0, width, height);
            // snapshot.append_color(&RGBA::new(0.8, 0.8, 0.8, 1.0), &background_rect);

            // Draw the progress bar itself
            let progress_rect = Rect::new(0.0, 0.0, progress_width, height);
            snapshot.append_color(&BAR_COLOR, &progress_rect);
        }
    }
}
