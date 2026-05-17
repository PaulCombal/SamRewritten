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

use gtk::ApplicationWindow;

#[cfg(unix)]
use gtk::glib::clone;
#[cfg(unix)]
use gtk::prelude::*;
#[cfg(unix)]
use gtk::{Align, Orientation, glib};

#[cfg(unix)]
fn show_markup_warning(parent: &ApplicationWindow, title: &str, markup: &str) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .destroy_with_parent(true)
        .default_width(520)
        .build();

    let label = gtk::Label::builder()
        .use_markup(true)
        .label(markup)
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .build();

    let close_button = gtk::Button::with_label("OK");
    close_button.add_css_class("suggested-action");
    close_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .margin_top(12)
        .build();
    button_box.append(&close_button);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(8)
        .build();
    content.append(&label);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

#[cfg(unix)]
pub fn warn(window: &ApplicationWindow) {
    use crate::utils::steam_locator::SteamLocator;

    let dirs = SteamLocator::get_local_steam_install_root_folders();
    if dirs.len() > 1 {
        let path_list = dirs
            .iter()
            .map(|p| format!("• {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        let full_message = format!(
            "Multiple Steam installations have been detected on your system. \
            This will most likely cause <b>severe instabilities</b> with SamRewritten. \
            <b>Please delete all unused Steam installations before proceeding or use \
            environment variables to point SamRewritten to the correct location, \
            as described on the <a href=\"https://github.com/PaulCombal/SamRewritten?tab=readme-ov-file#environment-variables\">Github main page.</a></b>\n\n\
            The following locations were found:\n{}",
            path_list
        );

        show_markup_warning(window, "WARNING", &full_message);
    } else if dirs.is_empty() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let flatpak_path = std::path::PathBuf::from(home)
            .join(".var/app/com.valvesoftware.Steam/.local/share/Steam");

        let full_message = if flatpak_path.exists() {
            "<b>We couldn't find a supported Steam installation on your system.</b>\n\n\
            It looks like you're using the <b>Flatpak</b> version of Steam. While Flatpak is great, \
            its security mechanisms prevents SamRewritten from talking to Steam safely.\n\n\
            <b>How to fix this:</b>\n\
            Please open your App Center (or Package Manager) and look for a different version of Steam \
            to install (common labels include 'System', 'Snap', 'package', or '.deb/.rpm').\n\n\
            Need help? Reach out to us on the <a href=\"https://github.com/PaulCombal/SamRewritten\">GitHub page.</a>"
        } else {
            "<b>No Steam installations were found on your system.</b>\n\n\
            SamRewritten couldn't find Steam in any of the standard locations. \
            If you haven't installed Steam yet, please install it through your \
            distribution's official repository or app store.\n\n\
            <b>Already have Steam installed?</b>\n\
            If you've installed Steam in a custom location, you can point SamRewritten \
            to it using environment variables. Please check the \
            <a href=\"https://github.com/PaulCombal/SamRewritten\">GitHub page</a> \
            for instructions, or to report your issue."
        };

        show_markup_warning(window, "No compatible version of Steam found", full_message);
    }
}

#[cfg(windows)]
pub fn warn(_window: &ApplicationWindow) {}
