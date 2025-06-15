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

#![windows_subsystem = "windows"]

mod backend;
mod frontend;
mod steam_client;
mod utils;

use std::process::Command;

use crate::backend::app::app;
use crate::backend::orchestrator::orchestrator;
use crate::utils::arguments::parse_arguments;
use frontend::main_ui;
use gtk::glib;
use utils::utils::get_executable_path;

const APP_ID: &str = "org.sam_authors.sam_rewritten";

fn main() -> glib::ExitCode {
    let arguments = parse_arguments();

    if arguments.is_orchestrator {
        let exit_code = orchestrator();
        return glib::ExitCode::from(exit_code);
    }

    if arguments.is_app > 0 {
        let exit_code = app(arguments.is_app);
        return glib::ExitCode::from(exit_code);
    }

    let current_exe = get_executable_path();
    let orchestrator = Command::new(current_exe)
        .arg("--orchestrator")
        .spawn()
        .expect("Failed to spawn sam2 orchestrator process");

    main_ui(orchestrator)
}
