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

mod achievement_automatic_view;
mod achievement_manual_view;
mod achievement_view;
mod app_list_view;
mod app_list_view_callbacks;
mod app_view;
mod application_actions;
mod custom_progress_bar_widget;
mod gobjects;
mod request;
mod stat_view;
mod ui_components;
mod widgets;

use crate::APP_ID;
use crate::gui_frontend::request::Request;
use crate::utils::bidir_child::BidirChild;
use app_list_view::create_main_ui;
use gtk::glib::ExitCode;
use gtk::prelude::{ApplicationExt, ApplicationExtManual};
use request::Shutdown;
use std::sync::RwLock;

static DEFAULT_PROCESS: RwLock<Option<BidirChild>> = RwLock::new(None);

fn shutdown() {
    if let Err(err) = Shutdown.request() {
        eprintln!("[CLIENT] Failed to send shutdown message: {}", err);
        return;
    };

    let mut guard = DEFAULT_PROCESS.write().unwrap();
    if let Some(ref mut bidir) = *guard {
        bidir
            .child
            .wait()
            .expect("[CLIENT] Failed to wait on orchestrator to shutdown");
    } else {
        panic!("[CLIENT] No orchestrator process to shutdown");
    }
}

#[cfg(not(feature = "adwaita"))]
pub type MainApplication = gtk::Application;
#[cfg(feature = "adwaita")]
pub type MainApplication = adw::Application;

pub fn main_ui(orchestrator: BidirChild) -> ExitCode {
    {
        let mut guard = DEFAULT_PROCESS.write().unwrap();
        *guard = Some(orchestrator);
    }

    let main_app = MainApplication::builder()
        .application_id(APP_ID)
        .flags(
            gtk::gio::ApplicationFlags::HANDLES_COMMAND_LINE
                | gtk::gio::ApplicationFlags::NON_UNIQUE,
        )
        .build();

    main_app.connect_command_line(create_main_ui);
    main_app.connect_shutdown(move |_| shutdown());
    main_app.run()
}
