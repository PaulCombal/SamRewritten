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
    compile_gschemas();
    compile_blueprints();
    pack_gresources();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}

fn pack_gresources() {
    glib_build_tools::compile_resources(
        &["assets"],
        "assets/org.samrewritten.SamRewritten.gresource.xml",
        "sam_rewritten.gresource",
    );
}

fn compile_blueprints() {
    let ui_dir = "assets/ui";

    // Tell Cargo to rerun this script if any blueprint file changes
    println!("cargo:rerun-if-changed={}", ui_dir);

    let entries = std::fs::read_dir(ui_dir).expect("Failed to read UI directory");

    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "blp") {
            let output_path = path.with_extension("ui");
            let status = Command::new("blueprint-compiler")
                .arg("batch-compile")
                .arg(ui_dir) // Output directory
                .arg(ui_dir) // Input directory
                .arg(&path)
                .status()
                .expect("Failed to execute blueprint-compiler. Is it installed?");

            if !status.success() {
                panic!("Blueprint compilation failed for {:?}", path);
            }
        }
    }
}

fn compile_gschemas() {
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
}
