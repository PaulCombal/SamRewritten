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


pub mod ipc_process;
mod request;
mod shimmer_image;
mod steam_app;
mod achievement;
mod app_view;
mod achievement_view;
mod app_list_view;
mod ui_components;
mod application_actions;
mod stat;
mod stat_view;
mod custom_progress_bar_widget;

use crate::{APP_ID, dev_println};
use std::cell::RefCell;
use std::process::Child;
use gtk::glib::ExitCode;
use gtk::Application;
use gtk::prelude::{ApplicationExt, ApplicationExtManual};
use request::Shutdown;
use app_list_view::create_main_ui;
use crate::frontend::request::Request;

fn shutdown(orchestrator: &RefCell<Child>) {
    Shutdown.request();

    match orchestrator.borrow_mut().wait() {
        Ok(code) => dev_println!("[CLIENT] Orchestrator process exited with: {code}"),
        Err(error) => dev_println!("[CLIENT] Failed to wait for orchestrator process: {error}"),
    }
}

pub fn main_ui(orchestrator: Child) -> ExitCode {
    let orchestrator = RefCell::new(orchestrator);
    let main_app = Application::builder().application_id(APP_ID).build();

    main_app.connect_activate(create_main_ui);
    main_app.connect_shutdown(move |_| shutdown(&orchestrator));
    main_app.run()
}
