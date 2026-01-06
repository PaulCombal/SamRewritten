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

use crate::gui_frontend::MainApplication;
use crate::gui_frontend::application_actions::set_app_action_enabled;
use gtk::gdk::Paintable;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::{BoxExt, ToVariant};
use gtk::{
    AboutDialog, ApplicationWindow, Image, Label, License, MenuButton, Popover, PopoverMenu,
    PositionType, Spinner, gdk_pixbuf,
};
use std::io::Cursor;

pub fn create_about_dialog(window: &ApplicationWindow) -> AboutDialog {
    let logo = load_logo();
    AboutDialog::builder()
        .modal(true)
        .transient_for(window)
        .hide_on_close(true)
        .license_type(License::Gpl30)
        .version(env!("CARGO_PKG_VERSION"))
        .program_name("SamRewritten")
        .authors(
            env!("CARGO_PKG_AUTHORS")
                .replace(" -@- ", "@")
                .split(':')
                .collect::<Vec<_>>(),
        )
        .comments(env!("CARGO_PKG_DESCRIPTION"))
        .logo(&logo)
        .build()
}

pub fn load_logo() -> Paintable {
    let image_bytes = include_bytes!("../../assets/icon_256.png");

    if let Ok(logo_pixbuf) = Pixbuf::from_read(Cursor::new(image_bytes)) {
        Image::from_pixbuf(Some(&logo_pixbuf))
            .paintable()
            .expect("Failed to create logo image")
    } else {
        eprintln!("[CLIENT] Failed to load logo. Using a gray square.");

        let pixbuf = Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1)
            .expect("Failed to create minimal pixbuf fallback");
        pixbuf.fill(0x808080FF);

        Image::from_pixbuf(Some(&pixbuf))
            .paintable()
            .expect("Failed to create logo image")
    }
}

pub fn create_context_menu_button() -> (
    MenuButton,
    PopoverMenu,
    gtk::gio::Menu,
    MenuButton,
    Label,
    Label,
) {
    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .build();

    let menu_button_loading_spinner = Spinner::builder().spinning(true).build();
    let menu_button_loading = MenuButton::builder()
        .child(&menu_button_loading_spinner)
        .visible(false)
        .build();

    let context_menu_model = gtk::gio::Menu::new();
    setup_app_list_popover_menu(&context_menu_model);

    let popover = PopoverMenu::builder()
        .position(PositionType::Bottom)
        .has_arrow(true)
        .menu_model(&context_menu_model)
        .build();

    let popover_loading_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(5)
        .margin_start(5)
        .margin_end(5)
        .margin_top(5)
        .margin_bottom(5)
        .width_request(200)
        .build();
    let popover_loading_progress_label = Label::new(Some("Loading..."));
    let popover_loading_info_label = Label::builder()
        .max_width_chars(20)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .build();
    popover_loading_box.append(&popover_loading_progress_label);
    popover_loading_box.append(&popover_loading_info_label);
    let popover_loading = Popover::builder().child(&popover_loading_box).build();

    menu_button.set_popover(Some(&popover));
    menu_button_loading.set_popover(Some(&popover_loading));

    (
        menu_button,
        popover,
        context_menu_model,
        menu_button_loading,
        popover_loading_progress_label,
        popover_loading_info_label,
    )
}

#[inline]
fn setup_app_list_popover_menu(menu_model: &gtk::gio::Menu) {
    menu_model.remove_all();
    let bulk_process_section = gtk::gio::Menu::new();
    bulk_process_section.append(Some("Select all visible apps"), Some("app.select_all_apps"));
    bulk_process_section.append(Some("Deselect all apps"), Some("app.unselect_all_apps"));
    bulk_process_section.append(Some("Unlock all in selection"), Some("app.unlock_all_apps"));
    bulk_process_section.append(Some("Reset all in selection"), Some("app.lock_all_apps"));

    menu_model.append(Some("Refresh app list"), Some("app.refresh_app_list"));
    let check_item = gtk::gio::MenuItem::new(Some("Filter junk"), Some("app.filter_junk_option"));
    menu_model.append_item(&check_item);
    menu_model.append(Some("About"), Some("app.about"));
    menu_model.append(Some("Quit"), Some("app.quit"));
    menu_model.append_section(Some("Bulk process (Beta)"), &bulk_process_section);

    let theme_section = gtk::gio::Menu::new();
    let theme_options = [
        #[cfg(feature = "adwaita")]
        ("System", "system"),
        ("Light", "light"),
        ("Dark", "dark"),
    ];

    for (label, value) in theme_options {
        let item = gtk::gio::MenuItem::new(Some(label), Some("app.change_theme"));
        item.set_action_and_target_value(Some("app.change_theme"), Some(&value.to_variant()));
        theme_section.append_item(&item);
    }

    menu_model.append_section(Some("Appearance"), &theme_section);
}

pub fn set_context_popover_to_app_list_context(
    menu_model: &gtk::gio::Menu,
    application: &MainApplication,
) {
    setup_app_list_popover_menu(menu_model);
    set_app_action_enabled(application, "refresh_achievements_list", false);
}

pub fn set_context_popover_to_app_details_context(
    menu_model: &gtk::gio::Menu,
    application: &MainApplication,
) {
    menu_model.remove_all();
    menu_model.append(
        Some("Refresh achievements & stats"),
        Some("app.refresh_achievements_list"),
    );
    menu_model.append(
        Some("Reset everything"),
        Some("app.clear_all_stats_and_achievements"),
    );
    menu_model.append(Some("About"), Some("app.about"));
    menu_model.append(Some("Quit"), Some("app.quit"));

    set_app_action_enabled(application, "refresh_app_list", false);
}
