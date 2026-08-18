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
use gtk::prelude::{BoxExt, SettingsExt, ToVariant};
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
        let base = crate::utils::app_paths::get_app_cache_dir().join("icons");
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
    setup_app_list_popover_menu(&context_menu_model, true);

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

/// `with_bulk` adds the bulk-process section, which acts on the app list's
/// selection. The profile page has no selection to act on, so it asks for the
/// same menu without it rather than offering entries that would silently do
/// nothing.
#[inline]
fn setup_app_list_popover_menu(menu_model: &gtk::gio::Menu, with_bulk: bool) {
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
    menu_model.append(
        Some(tr("Refresh achievement counts").as_str()),
        Some("app.rescan_achievement_counts"),
    );
    menu_model.append(Some(tr("About").as_str()), Some("app.about"));
    #[cfg(unix)]
    if crate::utils::snap::is_snap() {
        menu_model.append(
            Some(tr("Change Steam folder…").as_str()),
            Some("app.change-steam-folder"),
        );
    }

    // Sorting and the library filters live in the app list's sidebar now.
    if with_bulk {
        menu_model.append_section(Some(tr("Bulk process").as_str()), &bulk_process_section);
    }

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
    fill_language_menu(
        &language_menu,
        "app.app-language",
        &tr("System default"),
        crate::gui_frontend::i18n::LANGUAGES.iter().copied(),
    );
    menu_model.append_submenu(Some(&bilingual_label(tr_noop("Language"))), &language_menu);
}

/// Native names are deliberately untranslated, so a language stays findable
/// whatever the UI locale is.
fn fill_language_menu<'a>(
    menu: &gtk::gio::Menu,
    action: &str,
    default_label: &str,
    entries: impl Iterator<Item = (&'a str, &'a str)>,
) {
    menu.remove_all();

    let default_item = gtk::gio::MenuItem::new(Some(default_label), Some(action));
    default_item.set_action_and_target_value(Some(action), Some(&"".to_variant()));
    menu.append_item(&default_item);

    for (target, name) in entries {
        let item = gtk::gio::MenuItem::new(Some(name), Some(action));
        item.set_action_and_target_value(Some(action), Some(&target.to_variant()));
        menu.append_item(&item);
    }
}

/// Only for the menu that changes the UI language: if the app came up in a
/// language you can't read, the English half is how you find your way back.
/// Nothing else needs it — everywhere else the UI language is the one you asked
/// for.
fn bilingual_label(english: &str) -> String {
    let native = tr(english);
    if native == english {
        native.to_string()
    } else {
        format!("{native} • {english}")
    }
}

pub fn set_context_popover_to_app_list_context(
    menu_model: &gtk::gio::Menu,
    application: &MainApplication,
) {
    setup_app_list_popover_menu(menu_model, true);
    set_app_action_enabled(application, "refresh_achievements_list", false);
}

pub fn set_context_popover_to_profile_context(
    menu_model: &gtk::gio::Menu,
    application: &MainApplication,
) {
    setup_app_list_popover_menu(menu_model, false);
    set_app_action_enabled(application, "refresh_achievements_list", false);
}

thread_local! {
    /// Not threaded through the view: a game's languages are only known once its
    /// schema has been read, long after the menu is built.
    static ACHIEVEMENT_LANGUAGE_MENU: gtk::gio::Menu = gtk::gio::Menu::new();
    static ACHIEVEMENT_LANGUAGES_FROM_FETCH: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

/// Used until the fetch answers. The orchestrator may resolve a different Steam
/// install than this process can see, so its answer wins whenever there is one.
pub fn set_achievement_languages_provisional(languages: &[String]) {
    if !ACHIEVEMENT_LANGUAGES_FROM_FETCH.with(std::cell::Cell::get) {
        fill_achievement_language_menu(languages);
    }
}

/// `gio::Menu` is a live model, so an already-open popover picks this up.
pub fn set_achievement_languages(languages: &[String]) {
    // Empty means unreadable, which is no better than our own empty read.
    if languages.is_empty() {
        return;
    }
    ACHIEVEMENT_LANGUAGES_FROM_FETCH.with(|f| f.set(true));
    fill_achievement_language_menu(languages);
}

fn fill_achievement_language_menu(languages: &[String]) {
    use crate::gui_frontend::gsettings::get_settings;
    use crate::gui_frontend::i18n::STEAM_LANGUAGES;

    // Schemas disagree on casing ("LATAM" vs "latam") and the setting is shared
    // across games, so store the table's spelling and let the backend match loosely.
    let mut entries: Vec<(&str, &str)> = STEAM_LANGUAGES
        .iter()
        .filter(|(code, _)| languages.iter().any(|l| l.eq_ignore_ascii_case(code)))
        .map(|(code, name)| (*code, *name))
        .collect();
    entries.extend(
        languages
            .iter()
            .filter(|l| {
                !STEAM_LANGUAGES
                    .iter()
                    .any(|(code, _)| l.eq_ignore_ascii_case(code))
            })
            .map(|l| (l.as_str(), l.as_str())),
    );

    // A pick this game doesn't ship would leave the group with nothing selected,
    // reading as "no preference". Carry it on a permanently disabled action so it
    // draws selected but greyed. Exact match, as that is how GTK ticks a radio.
    let picked = get_settings().string("achievement-language").to_string();
    let unavailable = (!picked.is_empty() && !entries.iter().any(|(target, _)| *target == picked))
        .then(|| {
            STEAM_LANGUAGES
                .iter()
                .find(|(code, _)| code.eq_ignore_ascii_case(&picked))
                .map_or(picked.clone(), |(_, name)| (*name).to_owned())
        });

    ACHIEVEMENT_LANGUAGE_MENU.with(|menu| {
        fill_language_menu(
            menu,
            "app.achievement-language",
            &tr("Game default"),
            entries.into_iter(),
        );

        if let Some(name) = &unavailable {
            let action = "app.achievement-language-unavailable";
            let item = gtk::gio::MenuItem::new(Some(name), Some(action));
            item.set_action_and_target_value(Some(action), Some(&picked.to_variant()));
            menu.append_item(&item);
        }
    });
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

    ACHIEVEMENT_LANGUAGES_FROM_FETCH.with(|f| f.set(false));
    fill_achievement_language_menu(&[]);
    ACHIEVEMENT_LANGUAGE_MENU.with(|menu| {
        menu_model.append_submenu(Some(tr("Achievement language").as_str()), menu);
    });

    set_app_action_enabled(application, "refresh_app_list", false);
}
