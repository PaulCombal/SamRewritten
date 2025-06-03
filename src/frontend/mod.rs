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

mod achievement;
mod achievement_view;
mod app_list_view;
mod app_view;
mod application_actions;
mod custom_progress_bar_widget;
pub mod ipc_process;
mod request;
mod shimmer_image;
mod stat;
mod stat_view;
mod steam_app;
mod ui_components;

use crate::frontend::request::Request;
use crate::{APP_ID, dev_println};
use app_list_view::create_main_ui;
use gtk::glib::ExitCode;
use gtk::prelude::{ApplicationExt, ApplicationExtManual};
use request::Shutdown;
use std::cell::RefCell;
use std::process::Child;

fn shutdown(orchestrator: &RefCell<Child>) {
    match Shutdown.request() {
        Err(err) => {
            eprintln!("[CLIENT] Failed to send shutdown message: {}", err);
            return;
        }
        Ok(_) => {}
    };

    match orchestrator.borrow_mut().wait() {
        Ok(code) => dev_println!("[CLIENT] Orchestrator process exited with: {code}"),
        Err(error) => dev_println!("[CLIENT] Failed to wait for orchestrator process: {error}"),
    }
}

#[cfg(not(feature = "adwaita"))]
pub type MainApplication = gtk::Application;
#[cfg(feature = "adwaita")]
pub type MainApplication = adw::Application;

pub fn main_ui(orchestrator: Child) -> ExitCode {
    let orchestrator = RefCell::new(orchestrator);
    let main_app = MainApplication::builder().application_id(APP_ID).build();

    main_app.connect_activate(create_main_ui);
    main_app.connect_shutdown(move |_| shutdown(&orchestrator));
    main_app.run()
}
