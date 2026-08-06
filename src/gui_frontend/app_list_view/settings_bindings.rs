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
use crate::gui_frontend::app_list_view::FilterState;
use crate::gui_frontend::dialogs::show_message_dialog;
use crate::gui_frontend::i18n::{STEAM_LANGUAGES, tr, tr_noop};
use crate::gui_frontend::widgets::steam_app_card::ANIMATIONS_DISABLED;
use gtk::gio::{Settings, SimpleAction};
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{CustomFilter, CustomSorter, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;

/// Every boolean filter key the sidebar exposes. Kept in sync with `FILTERS` in
/// `sidebar.rs`, which owns their labels and ordering.
const FILTER_KEYS: &[&str] = &[
    "filter-junk",
    "filter-only-idling",
    "filter-hide-fully-unlocked",
    "filter-hide-never-launched",
    "filter-hide-no-unlocked",
    "filter-hide-without-achievements",
];

pub fn setup_settings_bindings(
    application: &MainApplication,
    settings: &Settings,
    list_custom_filter: &CustomFilter,
    list_custom_sorter: &CustomSorter,
    filter_state: Rc<FilterState>,
    sort_mode_cache: Rc<RefCell<String>>,
    on_filters_changed: Rc<dyn Fn()>,
) {
    // One handler per key rather than one for the lot: a `connect_changed`
    // with no key fires for every setting, including language and theme.
    for key in FILTER_KEYS {
        application.add_action(&settings.create_action(key));
        settings.connect_changed(
            Some(key),
            clone!(
                #[weak]
                list_custom_filter,
                #[strong]
                filter_state,
                #[strong]
                on_filters_changed,
                move |s, _| {
                    filter_state.reload(s);
                    list_custom_filter.changed(gtk::FilterChange::Different);
                    on_filters_changed();
                }
            ),
        );
    }

    // Sort radio: two-way bound to gsettings; re-sorts on any change.
    application.add_action(&settings.create_action("app-sort"));
    settings.connect_changed(
        Some("app-sort"),
        clone!(
            #[weak]
            list_custom_sorter,
            #[strong]
            on_filters_changed,
            move |s, _| {
                *sort_mode_cache.borrow_mut() = s.string("app-sort").to_string();
                list_custom_sorter.changed(gtk::SorterChange::Different);
                on_filters_changed();
            }
        ),
    );

    // Theme radio: two-way bound to gsettings; side effect (color scheme) via connect_changed.
    #[cfg(not(feature = "adwaita"))]
    {
        // Non-adwaita builds don't offer "System"; coerce the saved value once so a radio is selected.
        if settings.string("app-theme") == "system"
            && let Err(e) = settings.set_string("app-theme", "light")
        {
            eprintln!("[CLIENT] Error saving app-theme setting: {e:?}");
        }
    }

    application.add_action(&settings.create_action("app-theme"));

    #[cfg(feature = "adwaita")]
    fn apply_theme(name: &str) {
        let sm = adw::StyleManager::default();
        match name {
            "dark" => sm.set_color_scheme(adw::ColorScheme::PreferDark),
            "light" => sm.set_color_scheme(adw::ColorScheme::PreferLight),
            _ => sm.set_color_scheme(adw::ColorScheme::Default),
        }
    }

    #[cfg(not(feature = "adwaita"))]
    fn apply_theme(name: &str) {
        let s = gtk::Settings::default().expect("Could not get default settings");
        s.set_property("gtk-application-prefer-dark-theme", name == "dark");
    }

    apply_theme(&settings.string("app-theme"));
    settings.connect_changed(Some("app-theme"), |s, _| {
        apply_theme(&s.string("app-theme"));
    });

    // Language radio: applied at start-up by i18n::set_language; a runtime switch
    // would only half-update the UI, so just notify that a restart is needed.
    application.add_action(&settings.create_action("app-language"));
    settings.connect_changed(
        Some("app-language"),
        clone!(
            #[weak]
            application,
            move |_, _| {
                // English is the catalogue key, so reuse the literal for the
                // bilingual notice (no second lookup) unless we're already English.
                let english = tr_noop(
                    "The new language will be applied the next time you start SamRewritten.",
                );
                let native = tr(english);
                let detail = if native == english {
                    native.to_string()
                } else {
                    format!("{native}\n\n{english}")
                };
                show_message_dialog(
                    application.active_window().as_ref(),
                    &tr("Language changed"),
                    &detail,
                );
            }
        ),
    );

    // Menu targets and radio state are compared byte for byte, and this value can
    // arrive spelled differently, so settle on the table's spelling once.
    let picked = settings.string("achievement-language");
    if let Some((code, _)) = STEAM_LANGUAGES
        .iter()
        .find(|(code, _)| code.eq_ignore_ascii_case(&picked) && *code != picked)
        && let Err(e) = settings.set_string("achievement-language", code)
    {
        eprintln!("[CLIENT] Error normalising achievement-language setting: {e:?}");
    }

    // Achievement language radio: a plain stateful action rather than
    // Settings::create_action, because that one derives `enabled` from the key's
    // writability and we need to disable it during a timed unlock.
    let achievement_language = SimpleAction::new_stateful(
        "achievement-language",
        Some(&String::static_variant_type()),
        &settings.string("achievement-language").to_variant(),
    );
    achievement_language.connect_activate(clone!(
        #[strong]
        settings,
        move |action, target| {
            let Some(value) = target.and_then(|t| t.str()) else {
                return;
            };
            action.set_state(&value.to_variant());
            if let Err(e) = settings.set_string("achievement-language", value) {
                eprintln!("[CLIENT] Error saving achievement-language setting: {e:?}");
            }
        }
    ));
    application.add_action(&achievement_language);

    // Carries the menu's ghost row for a language the open game doesn't ship:
    // same state so it renders selected, never enabled so it renders greyed.
    let achievement_language_unavailable = SimpleAction::new_stateful(
        "achievement-language-unavailable",
        Some(&String::static_variant_type()),
        &settings.string("achievement-language").to_variant(),
    );
    achievement_language_unavailable.set_enabled(false);
    application.add_action(&achievement_language_unavailable);

    // Re-read the schema. Refresh is enabled exactly when that is possible: on an
    // app page, with no timed unlock running.
    settings.connect_changed(
        Some("achievement-language"),
        clone!(
            #[weak]
            application,
            #[strong]
            achievement_language,
            #[strong]
            achievement_language_unavailable,
            move |s, _| {
                // Also covers an external `gsettings set` and a second instance,
                // neither of which goes through the action.
                let value = s.string("achievement-language").to_variant();
                achievement_language.set_state(&value);
                achievement_language_unavailable.set_state(&value);
                if application
                    .lookup_action("refresh_achievements_list")
                    .and_then(|a| a.downcast::<SimpleAction>().ok())
                    .is_some_and(|a| a.is_enabled())
                {
                    application.activate_action("refresh_achievements_list", None);
                }
            }
        ),
    );

    // Disable animations: cached in a global AtomicBool that SteamAppCard reads.
    ANIMATIONS_DISABLED.store(settings.boolean("disable-animations"), Ordering::Relaxed);
    application.add_action(&settings.create_action("disable-animations"));
    settings.connect_changed(Some("disable-animations"), |s, _| {
        ANIMATIONS_DISABLED.store(s.boolean("disable-animations"), Ordering::Relaxed);
    });
}
