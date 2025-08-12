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

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]

mod backend;
mod frontend;
mod steam_client;
mod utils;

use std::process::Command;

use crate::backend::app::app;
use crate::backend::orchestrator::orchestrator;
use crate::utils::arguments::parse_cli_arguments;
use crate::utils::bidir_child::BidirChild;
use frontend::main_ui;
use gtk::glib;
use utils::app_paths::get_executable_path;

const APP_ID: &str = "org.samrewritten.SamRewritten";

fn main() -> glib::ExitCode {
    let arguments = parse_cli_arguments();

    if arguments.is_orchestrator {
        let mut tx = arguments.tx.unwrap();
        let mut rx = arguments.rx.unwrap();
        let exit_code = orchestrator(&mut tx, &mut rx);
        return glib::ExitCode::from(exit_code);
    }

    if arguments.is_app > 0 {
        let mut tx = arguments.tx.unwrap();
        let mut rx = arguments.rx.unwrap();
        let exit_code = app(arguments.is_app, &mut tx, &mut rx);
        return glib::ExitCode::from(exit_code);
    }

    let current_exe = get_executable_path();
    let orchestrator = BidirChild::new(Command::new(current_exe).arg("--orchestrator"))
        .expect("Failed to spawn orchestrator process");

    main_ui(orchestrator)
}
