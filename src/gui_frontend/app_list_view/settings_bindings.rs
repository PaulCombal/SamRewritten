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

use crate::gui_frontend::MainApplication;
use crate::gui_frontend::widgets::steam_app_card::ANIMATIONS_DISABLED;
use gtk::gio::Settings;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{CustomFilter, CustomSorter, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::Ordering;

pub fn setup_settings_bindings(
    application: &MainApplication,
    settings: &Settings,
    list_custom_filter: &CustomFilter,
    list_custom_sorter: &CustomSorter,
    filter_junk_cache: Rc<Cell<bool>>,
    sort_mode_cache: Rc<RefCell<String>>,
) {
    // Filter junk: two-way bound to gsettings; re-runs the filter on any change.
    application.add_action(&settings.create_action("filter-junk"));
    settings.connect_changed(
        Some("filter-junk"),
        clone!(
            #[weak]
            list_custom_filter,
            move |s, _| {
                filter_junk_cache.set(s.boolean("filter-junk"));
                list_custom_filter.changed(gtk::FilterChange::Different);
            }
        ),
    );

    // Sort radio: two-way bound to gsettings; re-sorts on any change.
    application.add_action(&settings.create_action("app-sort"));
    settings.connect_changed(
        Some("app-sort"),
        clone!(
            #[weak]
            list_custom_sorter,
            move |s, _| {
                *sort_mode_cache.borrow_mut() = s.string("app-sort").to_string();
                list_custom_sorter.changed(gtk::SorterChange::Different);
            }
        ),
    );

    // Theme radio: two-way bound to gsettings; side effect (color scheme) via connect_changed.
    #[cfg(not(feature = "adwaita"))]
    {
        // Non-adwaita builds don't offer "System"; coerce the saved value once so a radio is selected.
        if settings.string("app-theme") == "system"
            && let Err(e) = settings.set_string("app-theme", "light")
        {
            eprintln!("[CLIENT] Error saving app-theme setting: {e:?}");
        }
    }

    application.add_action(&settings.create_action("app-theme"));

    #[cfg(feature = "adwaita")]
    fn apply_theme(name: &str) {
        let sm = adw::StyleManager::default();
        match name {
            "dark" => sm.set_color_scheme(adw::ColorScheme::PreferDark),
            "light" => sm.set_color_scheme(adw::ColorScheme::PreferLight),
            _ => sm.set_color_scheme(adw::ColorScheme::Default),
        }
    }

    #[cfg(not(feature = "adwaita"))]
    fn apply_theme(name: &str) {
        let s = gtk::Settings::default().expect("Could not get default settings");
        s.set_property("gtk-application-prefer-dark-theme", name == "dark");
    }

    apply_theme(&settings.string("app-theme"));
    settings.connect_changed(Some("app-theme"), |s, _| {
        apply_theme(&s.string("app-theme"));
    });

    // Disable animations: cached in a global AtomicBool that SteamAppCard reads.
    ANIMATIONS_DISABLED.store(settings.boolean("disable-animations"), Ordering::Relaxed);
    application.add_action(&settings.create_action("disable-animations"));
    settings.connect_changed(Some("disable-animations"), |s, _| {
        ANIMATIONS_DISABLED.store(s.boolean("disable-animations"), Ordering::Relaxed);
    });
}
