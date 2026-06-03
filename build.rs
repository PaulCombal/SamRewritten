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

/// Compile each `po/<lang>.po` to `locale/<lang>/LC_MESSAGES/samrewritten.mo`.
/// Best-effort: a missing `msgfmt` drops translations rather than failing.
fn compile_translations() {
    let po_dir = std::path::Path::new("po");
    println!("cargo:rerun-if-changed=po");
    let Ok(entries) = std::fs::read_dir(po_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("po") {
            continue;
        }
        let Some(lang) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        let out_dir = format!("locale/{lang}/LC_MESSAGES");
        if std::fs::create_dir_all(&out_dir).is_err() {
            continue;
        }
        let out_file = format!("{out_dir}/samrewritten.mo");

        match Command::new("msgfmt")
            .arg("--output-file")
            .arg(&out_file)
            .arg(&path)
            .status()
        {
            Ok(s) if s.success() => {
                println!("cargo:rerun-if-changed=po/{lang}.po");
            }
            Ok(s) => println!("cargo:warning=msgfmt failed for {lang}.po (exit {s})"),
            Err(e) => println!("cargo:warning=skipping translations, msgfmt unavailable: {e}"),
        }
    }
}

fn main() {
    // Always present so the AppImage `assets` list resolves on non-GUI builds too.
    let _ = std::fs::create_dir_all("locale");

    if std::env::var_os("CARGO_FEATURE_GUI").is_some() {
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

        compile_translations();
    }

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}
