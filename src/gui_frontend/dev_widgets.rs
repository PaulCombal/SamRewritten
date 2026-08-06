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

//! Debug-build only: with `SAM_DEV_DUMP_WIDGETS=/path` set, writes the live
//! widget tree and each widget's box to that file once a second, so the window
//! can be driven from outside by grepping for a label.
//!
//! Coordinates are relative to the window's own top-left. `xdotool` wants them
//! relative to the *X* window, which under client-side decorations includes the
//! invisible shadow border, so add half the difference between the two sizes.

use gtk::prelude::*;
use gtk::{ApplicationWindow, Widget, glib};
use std::fmt::Write as _;

const DUMP_INTERVAL_SECS: u32 = 1;

pub(super) fn install(window: &ApplicationWindow) {
    let Ok(path) = std::env::var("SAM_DEV_DUMP_WIDGETS") else {
        return;
    };
    eprintln!("[DEV] Dumping the widget tree to {path} every {DUMP_INTERVAL_SECS}s");

    glib::timeout_add_seconds_local(
        DUMP_INTERVAL_SECS,
        glib::clone!(
            #[weak]
            window,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move || {
                let mut out = format!(
                    "# window {}x{}, coordinates are window-relative\n\
                     # centre-x,centre-y  width x height  type  [name]  \"text\"\n",
                    window.width(),
                    window.height()
                );
                if let Some(root) = window.child() {
                    describe(&root, &window, 0, &mut out);
                }
                if let Err(e) = std::fs::write(&path, out) {
                    eprintln!("[DEV] Could not write {path}: {e}");
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            }
        ),
    );
}

fn describe(widget: &Widget, window: &ApplicationWindow, depth: usize, out: &mut String) {
    if widget.is_mapped()
        && let Some(bounds) = widget.compute_bounds(window)
    {
        let name = widget.widget_name();
        let name = if name == widget.type_().name() {
            String::new()
        } else {
            format!("  [{name}]")
        };
        let _ = writeln!(
            out,
            "{:indent$}{cx},{cy}  {w}x{h}  {ty}{name}{text}",
            "",
            indent = depth * 2,
            cx = (bounds.x() + bounds.width() / 2.0).round() as i32,
            cy = (bounds.y() + bounds.height() / 2.0).round() as i32,
            w = bounds.width().round() as i32,
            h = bounds.height().round() as i32,
            ty = widget.type_().name(),
            text = text_of(widget),
        );
    }

    let mut child = widget.first_child();
    while let Some(this) = child {
        describe(&this, window, depth + 1, out);
        child = this.next_sibling();
    }
}

fn text_of(widget: &Widget) -> String {
    let text = if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        Some(label.label().to_string())
    } else if let Some(button) = widget.downcast_ref::<gtk::Button>() {
        button.label().map(|l| l.to_string())
    } else if let Some(check) = widget.downcast_ref::<gtk::CheckButton>() {
        check.label().map(|l| l.to_string())
    } else if let Some(entry) = widget.downcast_ref::<gtk::SearchEntry>() {
        entry.placeholder_text().map(|t| t.to_string())
    } else {
        None
    };
    match text {
        Some(text) if !text.is_empty() => format!("  \"{text}\""),
        _ => String::new(),
    }
}
