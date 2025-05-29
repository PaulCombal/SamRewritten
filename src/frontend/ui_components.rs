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


use gtk::gdk_pixbuf::Pixbuf;
use gtk::{AboutDialog, ApplicationWindow, Image, License, MenuButton, PopoverMenu, PositionType};
use std::io::Cursor;
use gtk::gdk::Paintable;
use crate::frontend::application_actions::set_app_action_enabled;

pub fn create_about_dialog(window: &ApplicationWindow, logo: &Paintable) -> AboutDialog {
    AboutDialog::builder()
        .modal(true)
        .transient_for(window)
        .hide_on_close(true)
        .license_type(License::Gpl30)
        .version(env!("CARGO_PKG_VERSION"))
        .program_name("SamRewritten")
        .authors(env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<_>>())
        .comments(env!("CARGO_PKG_DESCRIPTION"))
        .logo(logo)
        .build()
}

pub fn load_logo() -> Paintable {
    // TODO: See if the forward slash syntax works on both?
    #[cfg(target_os = "windows")]
    let image_bytes = include_bytes!("..\\..\\assets\\icon_256.png");
    #[cfg(target_os = "linux")]
    let image_bytes = include_bytes!("../../assets/icon_256.png");

    let logo_pixbuf = Pixbuf::from_read(Cursor::new(image_bytes)).expect("Failed to load logo");
    Image::from_pixbuf(Some(&logo_pixbuf)).paintable().expect("Failed to create logo image")
}

pub fn create_context_menu_button() -> (MenuButton, PopoverMenu, gtk::gio::Menu) {
    let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").build();

    let context_menu_model = gtk::gio::Menu::new();

    // Let's remember we can add sections, but for now I don't see the use case
    // let section = gio::Menu::new();
    // section.append(Some("Sub Item A"), Some("app.subitemA"));
    // menu.append_section(Some("Section"), &section);
    context_menu_model.append(Some("Refresh app list"), Some("app.refresh_app_list"));
    context_menu_model.append(Some("About"), Some("app.about"));
    context_menu_model.append(Some("Quit"), Some("app.quit"));

    let popover = PopoverMenu::builder()
        .position(PositionType::Bottom)
        .has_arrow(true)
        .menu_model(&context_menu_model)
        .build();

    menu_button.set_popover(Some(&popover));

    (
        menu_button,
        popover,
        context_menu_model,
    )
}

pub fn set_context_popover_to_app_list_context(menu_model: &gtk::gio::Menu, application: &gtk::Application) {
    menu_model.remove_all();
    menu_model.append(Some("Refresh app list"), Some("app.refresh_app_list"));
    menu_model.append(Some("About"), Some("app.about"));
    menu_model.append(Some("Quit"), Some("app.quit"));
    
    set_app_action_enabled(&application, "refresh_achievements_list", false);
}

pub fn set_context_popover_to_app_details_context(menu_model: &gtk::gio::Menu, application: &gtk::Application) {
    menu_model.remove_all();
    menu_model.append(Some("Refresh achievements & stats"), Some("app.refresh_achievements_list"));
    menu_model.append(Some("About"), Some("app.about"));
    menu_model.append(Some("Quit"), Some("app.quit"));

    set_app_action_enabled(&application, "refresh_app_list", false);
}
