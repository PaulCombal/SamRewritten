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

use std::process::Command;

fn main() {
    let schema_dir = "assets";

    let status = Command::new("glib-compile-schemas")
        .arg(schema_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:rerun-if-changed={}", schema_dir);
        }
        Ok(s) => {
            panic!("glib-compile-schemas failed with exit code: {}", s);
        }
        Err(e) => {
            panic!(
                "Failed to execute glib-compile-schemas: {}. \
                Make sure GLib development tools are installed.",
                e
            );
        }
    }

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}
