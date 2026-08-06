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

//! Holds a child to a maximum width and centres it, leaving gutters either side.
//!
//! Libadwaita ships `AdwClamp`, but the default build has no libadwaita, and one
//! widget both builds share beats the same page laid out two ways.

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct Clamp(ObjectSubclass<imp::Clamp>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Clamp {
    pub fn new(child: &impl IsA<gtk::Widget>, maximum_size: i32) -> Self {
        let obj: Self = glib::Object::new();
        let child = child.as_ref();
        obj.imp().maximum_size.set(maximum_size);
        child.set_parent(&obj);
        obj.imp().child.replace(Some(child.clone()));
        obj
    }
}

mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{Orientation, glib};
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub struct Clamp {
        pub(super) child: RefCell<Option<gtk::Widget>>,
        pub(super) maximum_size: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Clamp {
        const NAME: &'static str = "SamClamp";
        type Type = super::Clamp;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Clamp {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Clamp {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            match self.child.borrow().as_ref() {
                Some(child) => child.request_mode(),
                None => gtk::SizeRequestMode::ConstantSize,
            }
        }

        fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let Some(child) = self.child.borrow().clone() else {
                return (0, 0, -1, -1);
            };
            let maximum = self.maximum_size.get();

            if orientation == Orientation::Horizontal {
                let (min, nat, _, _) = child.measure(orientation, for_size);
                return (min, nat.min(maximum).max(min), -1, -1);
            }
            // For the width the child will get, not the one we were handed.
            let width = if for_size < 0 {
                -1
            } else {
                for_size.min(maximum)
            };
            let (min, nat, _, _) = child.measure(orientation, width);
            (min, nat, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let Some(child) = self.child.borrow().clone() else {
                return;
            };
            let child_width = width.min(self.maximum_size.get());
            let x = (width - child_width) / 2;
            child.size_allocate(&gtk::Allocation::new(x, 0, child_width, height), baseline);
        }
    }
}
