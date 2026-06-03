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
use crate::gui_frontend::i18n::{tr, tr_noop};
use gtk::prelude::{BoxExt, ToVariant};
use gtk::{Label, License, MenuButton, Popover, PopoverMenu, PositionType, Spinner};

#[cfg(not(feature = "adwaita"))]
use gtk::AboutDialog;
#[cfg(not(feature = "adwaita"))]
use gtk::gdk::Paintable;
#[cfg(not(feature = "adwaita"))]
use gtk::gdk_pixbuf::{self, Pixbuf};
#[cfg(not(feature = "adwaita"))]
use gtk::glib::object::Cast;
#[cfg(not(feature = "adwaita"))]
use gtk::prelude::GtkWindowExt;
#[cfg(not(feature = "adwaita"))]
use std::io::Cursor;

#[cfg(feature = "adwaita")]
pub fn show_about_dialog(parent: Option<&gtk::Window>) {
    use adw::prelude::*;

    register_app_icon();
    adw::AboutDialog::builder()
        .application_name("SamRewritten")
        .application_icon(crate::APP_ID)
        .version(env!("CARGO_PKG_VERSION"))
        .developers(
            env!("CARGO_PKG_AUTHORS")
                .replace(" -@- ", "@")
                .split(':')
                .collect::<Vec<_>>(),
        )
        .comments(env!("CARGO_PKG_DESCRIPTION"))
        .license_type(License::Gpl30)
        .build()
        .present(parent);
}

// adw's AboutDialog takes a themed icon *name*, not a paintable, and we ship no
// app icon on the theme path. Drop the embedded PNG into a cache icon dir so
// `application_icon(APP_ID)` resolves (dev runs and packaged builds alike).
#[cfg(feature = "adwaita")]
fn register_app_icon() {
    use std::sync::Once;
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        let theme = gtk::IconTheme::for_display(&display);
        if theme.has_icon(crate::APP_ID) {
            return;
        }
        let base = gtk::glib::user_cache_dir().join("samrewritten/icons");
        let apps = base.join("hicolor/256x256/apps");
        let icon = apps.join(format!("{}.png", crate::APP_ID));
        if !icon.exists()
            && let Err(e) = std::fs::create_dir_all(&apps)
                .and_then(|()| std::fs::write(&icon, include_bytes!("../../assets/icon_256.png")))
        {
            crate::dev_println!("CLIENT", "Could not stage About icon: {e}");
            return;
        }
        theme.add_search_path(&base);
    });
}

#[cfg(not(feature = "adwaita"))]
pub fn show_about_dialog(parent: Option<&gtk::Window>) {
    let logo = load_logo();
    let dialog = AboutDialog::builder()
        .modal(true)
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
        .build();
    dialog.set_transient_for(parent);
    dialog.present();
}

#[cfg(not(feature = "adwaita"))]
pub fn load_logo() -> Paintable {
    let image_bytes = include_bytes!("../../assets/icon_256.png");

    if let Ok(logo_pixbuf) = Pixbuf::from_read(Cursor::new(image_bytes)) {
        gtk::gdk::Texture::for_pixbuf(&logo_pixbuf).upcast::<Paintable>()
    } else {
        eprintln!("[CLIENT] Failed to load logo. Using a gray square.");

        let pixbuf = Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1)
            .expect("Failed to create minimal pixbuf fallback");
        pixbuf.fill(0x808080FF);

        gtk::gdk::Texture::for_pixbuf(&pixbuf).upcast::<Paintable>()
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
    let popover_loading_progress_label = Label::new(Some(tr("Loading...").as_str()));
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
    bulk_process_section.append(
        Some(tr("Select all visible apps").as_str()),
        Some("app.select_all_apps"),
    );
    bulk_process_section.append(
        Some(tr("Deselect all apps").as_str()),
        Some("app.unselect_all_apps"),
    );
    bulk_process_section.append(
        Some(tr("Unlock all in selection").as_str()),
        Some("app.unlock_all_apps"),
    );
    bulk_process_section.append(
        Some(tr("Reset all in selection").as_str()),
        Some("app.lock_all_apps"),
    );
    bulk_process_section.append(
        Some(tr("Export selected apps progress").as_str()),
        Some("app.export_selected_progress"),
    );
    bulk_process_section.append(
        Some(tr("Import progress...").as_str()),
        Some("app.import_progress"),
    );

    menu_model.append(
        Some(tr("Refresh app list").as_str()),
        Some("app.refresh_app_list"),
    );
    let check_item =
        gtk::gio::MenuItem::new(Some(tr("Filter junk").as_str()), Some("app.filter-junk"));
    menu_model.append_item(&check_item);
    menu_model.append(Some(tr("About").as_str()), Some("app.about"));

    let sort_section = gtk::gio::Menu::new();
    // tr_noop marks labels for extraction; the second element is the action target.
    let sort_options = [
        (tr_noop("App ID"), "app_id"),
        (tr_noop("Alphabetical"), "alphabetical"),
        (tr_noop("Recently played"), "last_played"),
        (tr_noop("Time played"), "playtime"),
    ];
    for (label, value) in sort_options {
        let item = gtk::gio::MenuItem::new(Some(tr(label).as_str()), Some("app.app-sort"));
        item.set_action_and_target_value(Some("app.app-sort"), Some(&value.to_variant()));
        sort_section.append_item(&item);
    }
    menu_model.append_section(Some(tr("Sort by").as_str()), &sort_section);

    menu_model.append_section(Some(tr("Bulk process").as_str()), &bulk_process_section);

    let theme_section = gtk::gio::Menu::new();
    let theme_options = [
        #[cfg(feature = "adwaita")]
        (tr_noop("System"), "system"),
        (tr_noop("Light"), "light"),
        (tr_noop("Dark"), "dark"),
    ];

    for (label, value) in theme_options {
        let item = gtk::gio::MenuItem::new(Some(tr(label).as_str()), Some("app.app-theme"));
        item.set_action_and_target_value(Some("app.app-theme"), Some(&value.to_variant()));
        theme_section.append_item(&item);
    }
    theme_section.append(
        Some(tr("Disable animations").as_str()),
        Some("app.disable-animations"),
    );

    menu_model.append_section(Some(tr("Appearance").as_str()), &theme_section);

    let language_menu = gtk::gio::Menu::new();
    // Empty target = follow system locale; native names are intentionally untranslated.
    let system_item = gtk::gio::MenuItem::new(
        Some(tr("System default").as_str()),
        Some("app.app-language"),
    );
    system_item.set_action_and_target_value(Some("app.app-language"), Some(&"".to_variant()));
    language_menu.append_item(&system_item);
    for (code, name) in crate::gui_frontend::i18n::LANGUAGES {
        let item = gtk::gio::MenuItem::new(Some(name), Some("app.app-language"));
        item.set_action_and_target_value(Some("app.app-language"), Some(&code.to_variant()));
        language_menu.append_item(&item);
    }
    let english = tr_noop("Language");
    let native = tr(english);
    let language_label = if native == english {
        native.to_string()
    } else {
        format!("{native} • {english}")
    };
    menu_model.append_submenu(Some(&language_label), &language_menu);
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
        Some(tr("Refresh achievements & stats").as_str()),
        Some("app.refresh_achievements_list"),
    );
    menu_model.append(
        Some(tr("Reset everything").as_str()),
        Some("app.clear_all_stats_and_achievements"),
    );
    menu_model.append(Some(tr("About").as_str()), Some("app.about"));

    set_app_action_enabled(application, "refresh_app_list", false);
}
