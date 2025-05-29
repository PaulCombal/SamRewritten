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

use gtk::gio::SimpleAction;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{AboutDialog, Application};
use gtk::glib;

pub fn setup_app_actions(
    application: &Application,
    about_dialog: &AboutDialog,
    refresh_app_list_action: &SimpleAction,
    refresh_achievements_list_action: &SimpleAction,
) {
    let action_show_about_dialog = SimpleAction::new("about", None);
    action_show_about_dialog.connect_activate(clone!(#[weak] about_dialog, move |_,_| {
         about_dialog.show();
    }));

    let action_quit = SimpleAction::new("quit", None);
    action_quit.connect_activate(clone!(#[weak] application, move |_,_| {
         application.quit();
    }));

    application.add_action(refresh_app_list_action);
    application.add_action(refresh_achievements_list_action);
    application.add_action(&action_show_about_dialog);
    application.add_action(&action_quit);
    application.set_accels_for_action("app.refresh_app_list", &["F5"]);
    application.set_accels_for_action("app.refresh_achievements_list", &["F5"]);
}

pub fn set_app_action_enabled(application: &Application, action_name: &str, enabled: bool) {
    if let Some(action) = application.lookup_action(action_name) {
        action.downcast_ref::<SimpleAction>().unwrap().set_enabled(enabled);
    }
}
