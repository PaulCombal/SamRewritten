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

use crate::gui_frontend::i18n::tr;
use gtk::ApplicationWindow;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{Align, Orientation};

#[cfg(unix)]
use std::cell::Cell;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::rc::Rc;

/// Scrollable, selectable, copyable list dialog. Use when the body may contain
/// more than ~10 entries — the plain `AlertDialog` detail string can't scroll.
/// `intro` is a static header above the scroll area; pass `""` to omit.
pub fn show_list_dialog(
    parent: &impl gtk::glib::object::IsA<gtk::Window>,
    title: &str,
    intro: &str,
    body: &str,
) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .destroy_with_parent(true)
        .default_width(560)
        .default_height(420)
        .build();

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(12)
        .build();

    if !intro.is_empty() {
        let intro_label = gtk::Label::builder()
            .label(intro)
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build();
        content.append(&intro_label);
    }

    let text_view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(8)
        .right_margin(8)
        .build();
    text_view.buffer().set_text(body);

    let scroller = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .min_content_height(200)
        .propagate_natural_height(true)
        .has_frame(true)
        .child(&text_view)
        .build();
    content.append(&scroller);

    let ok_button = gtk::Button::with_label(tr("OK").as_str());
    ok_button.add_css_class("suggested-action");
    ok_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .build();
    button_box.append(&ok_button);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

#[cfg(feature = "adwaita")]
pub fn show_message_dialog(parent: Option<&gtk::Window>, title: &str, body: &str) {
    use adw::prelude::*;

    let dialog = adw::AlertDialog::new(Some(title), Some(body));
    dialog.add_response("ok", tr("OK").as_str());
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("ok");
    dialog.present(parent);
}

#[cfg(not(feature = "adwaita"))]
pub fn show_message_dialog(parent: Option<&gtk::Window>, title: &str, body: &str) {
    let dialog = gtk::Window::builder()
        .modal(true)
        .title(title)
        .resizable(false)
        .destroy_with_parent(true)
        .default_width(380)
        .build();
    dialog.set_transient_for(parent);

    let label = gtk::Label::builder()
        .label(body)
        .wrap(true)
        .xalign(0.0)
        .build();

    let ok_button = gtk::Button::with_label(tr("OK").as_str());
    ok_button.add_css_class("suggested-action");
    ok_button.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .margin_top(12)
        .build();
    button_box.append(&ok_button);

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

    let close_button = gtk::Button::with_label(tr("OK").as_str());
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

/// Pick the Steam install once, before the main window, then call `on_chosen`
/// (`None` = locator default). Runs on every dismissal path so the caller always
/// gets to start up.
#[cfg(unix)]
pub fn choose_steam_install_then<F>(parent: &ApplicationWindow, on_chosen: F)
where
    F: Fn(Option<PathBuf>) + 'static,
{
    use crate::utils::steam_locator::SteamLocator;

    let dirs = SteamLocator::get_local_steam_install_root_folders();

    if dirs.is_empty() {
        let full_message = format!(
            "{}\n\n{}\n\n{}\n{}",
            tr("<b>No Steam installations were found on your system.</b>"),
            tr(
                "SamRewritten couldn't find Steam in any of the standard locations. If you haven't installed Steam yet, please install it through your distribution's official repository or app store."
            ),
            tr("<b>Already have Steam installed?</b>"),
            tr(
                "If you've installed Steam in a custom location, you can point SamRewritten to it using environment variables. Please check the <a href=\"https://github.com/PaulCombal/SamRewritten\">GitHub page</a> for instructions, or to report your issue."
            ),
        );
        show_markup_warning(
            parent,
            tr("No compatible version of Steam found").as_str(),
            full_message.as_str(),
        );
        on_chosen(None);
        return;
    }

    if dirs.len() == 1 {
        on_chosen(None);
        return;
    }

    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(tr("Choose a Steam installation").as_str())
        .destroy_with_parent(true)
        .default_width(560)
        .build();

    let intro = gtk::Label::builder()
        .label(
            tr("SamRewritten found more than one Steam installation. The one Steam is currently running from is preselected — the others won't work unless you start Steam from them first.")
            .as_str(),
        )
        .wrap(true)
        .xalign(0.0)
        .build();

    let radio_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    // Only the install Steam is running from works; preselect and flag it.
    let running = crate::utils::steam_ns::running_steam_install_roots();
    let is_running = |dir: &std::path::Path| {
        std::fs::canonicalize(dir)
            .map(|c| running.contains(&c))
            .unwrap_or(false)
    };
    let default_idx = dirs.iter().position(|d| is_running(d)).unwrap_or(0);

    let buttons: Vec<gtk::CheckButton> = dirs
        .iter()
        .map(|dir| {
            let suffix = if is_running(dir) {
                tr("    (Steam is running here)")
            } else {
                tr("    (Steam not running here)")
            };
            let cb = gtk::CheckButton::with_label(&format!("{}{suffix}", dir.display()));
            radio_box.append(&cb);
            cb
        })
        .collect();
    for cb in buttons.iter().skip(1) {
        cb.set_group(Some(&buttons[0]));
    }
    buttons[default_idx].set_active(true);

    let hint = gtk::Label::builder()
        .use_markup(true)
        .label(
            tr("You'll be asked again next launch. To skip this for good, set the <tt>SAM_STEAM_INSTALL_ROOT</tt> environment variable to the install you want — see the <a href=\"https://github.com/PaulCombal/SamRewritten?tab=readme-ov-file#environment-variables\">README</a>.")
            .as_str(),
        )
        .wrap(true)
        .xalign(0.0)
        .build();

    let dirs = Rc::new(dirs);
    let buttons = Rc::new(buttons);
    let on_chosen = Rc::new(on_chosen);
    let confirmed = Rc::new(Cell::new(false));

    // Confirming starts the app on the selected install; closing the window any
    // other way cancels and quits (the main window is never shown).
    dialog.connect_close_request(clone!(
        #[weak]
        parent,
        #[strong]
        buttons,
        #[strong]
        dirs,
        #[strong]
        on_chosen,
        #[strong]
        confirmed,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_| {
            if confirmed.get() {
                let idx = buttons
                    .iter()
                    .position(gtk::CheckButton::is_active)
                    .unwrap_or(0);
                on_chosen(Some(dirs[idx].clone()));
            } else if let Some(app) = parent.application() {
                app.quit();
            }
            glib::Propagation::Proceed
        }
    ));

    let use_button = gtk::Button::with_label(tr("Use this installation").as_str());
    use_button.add_css_class("suggested-action");
    use_button.connect_clicked(clone!(
        #[weak]
        dialog,
        #[strong]
        confirmed,
        move |_| {
            confirmed.set(true);
            dialog.close();
        }
    ));

    let button_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::End)
        .spacing(8)
        .margin_top(12)
        .build();
    button_box.append(&use_button);

    let content = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .spacing(12)
        .build();
    content.append(&intro);
    content.append(&radio_box);
    content.append(&hint);
    content.append(&button_box);

    dialog.set_child(Some(&content));
    dialog.present();
}

#[cfg(windows)]
pub fn choose_steam_install_then<F>(_parent: &ApplicationWindow, on_chosen: F)
where
    F: Fn(Option<std::path::PathBuf>) + 'static,
{
    on_chosen(None);
}
