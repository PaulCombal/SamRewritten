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

use crate::backend::local_stats::read_schema_languages;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::app_list_view::PrefetchedProgress;
use crate::gui_frontend::application_actions::set_app_action_enabled;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::request::{GetAchievementsAndStats, Request};
use crate::gui_frontend::ui_components::{
    set_achievement_languages_provisional, set_context_popover_to_app_details_context,
};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use crate::utils::format::format_playtime_minutes;
use gtk::gio::{Menu, Settings, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::{Box, Label, Stack};
use std::cell::Cell;
use std::rc::Rc;

#[inline]
pub(crate) fn switch_from_app_list_to_app(
    steam_app_object: &GSteamAppObject,
    application: MainApplication,
    app_type_value_label: &Label,
    app_developer_value_label: &Label,
    app_achievement_count_value_label: &Label,
    app_stats_count_value_label: &Label,
    app_stack: Stack,
    app_id: &Rc<Cell<Option<u32>>>,
    app_metacritic_box: &Box,
    app_metacritic_value_label: &Label,
    app_playtime_value_label: &Label,
    app_shimmer_image: &ShimmerImage,
    app_label: &Label,
    menu_model: &Menu,
    list_stack: &Stack,
    prefetched_progress: &PrefetchedProgress,
    settings: &Settings,
) {
    set_app_action_enabled(&application, "refresh_achievements_list", false);
    app_type_value_label.set_label(&steam_app_object.app_type());
    app_developer_value_label.set_label(&steam_app_object.developer());
    app_achievement_count_value_label.set_label("...");
    app_stats_count_value_label.set_label("...");
    app_stack.set_visible_child_name("loading");
    app_id.set(Some(steam_app_object.app_id()));
    app_metacritic_box.set_visible(steam_app_object.metacritic_score() != u8::MAX);
    app_metacritic_value_label.set_label(&format!("{}", steam_app_object.metacritic_score()));
    app_playtime_value_label.set_label(&format_playtime_minutes(
        steam_app_object.playtime_minutes(),
    ));

    if let Some(url) = steam_app_object.image_url() {
        app_shimmer_image.set_url(url.as_str());
    } else {
        app_shimmer_image.reset();
    }

    app_label.set_markup(&format!(
        "<span font_desc=\"Bold 16\">{}</span>",
        gtk::glib::markup_escape_text(&steam_app_object.app_name())
    ));

    let app_id_copy = steam_app_object.app_id();
    let language = settings.string("achievement-language").to_string();
    let requested_language = language.clone();
    let handle = spawn_blocking(move || {
        GetAchievementsAndStats {
            app_id: app_id_copy,
            launch: true,
            language,
        }
        .request()
    });

    set_context_popover_to_app_details_context(menu_model, &application);

    // This depends only on the schema on disk, so don't wait on the fetch that also
    // carries it: a game that fails to launch never delivers one. An empty read means
    // we couldn't see the file, and the fetch's answer is the better one anyway.
    let languages = spawn_blocking(move || read_schema_languages(app_id_copy));
    MainContext::default().spawn_local(clone!(
        #[strong]
        app_id,
        async move {
            if let Ok(languages) = languages.await
                && !languages.is_empty()
                && app_id.get() == Some(app_id_copy)
            {
                set_achievement_languages_provisional(&languages);
            }
        }
    ));

    let prefetched_progress = prefetched_progress.clone();
    MainContext::default().spawn_local(clone!(
        #[strong]
        app_id,
        async move {
            // Launch and the achievement/stats fetch ride in one round-trip; hand
            // the result to the refresh action so no second fetch is needed.
            let result = handle.await;

            // Another game may own the page by now; the widgets below are shared
            // with it, and the refresh this kicks off would collide with its own.
            if app_id.get() != Some(app_id_copy) {
                return;
            }

            match result {
                Ok(Ok(progress)) => {
                    prefetched_progress.replace(Some((app_id_copy, requested_language, progress)))
                }
                other => {
                    eprintln!("[LAUNCH APP] Failed to launch app: {other:?}");
                    return app_stack.set_visible_child_name("failed");
                }
            };

            set_app_action_enabled(&application, "refresh_achievements_list", true);
            set_app_action_enabled(&application, "clear_all_stats_and_achievements", true);

            application.activate_action("refresh_achievements_list", None);
        }
    ));

    list_stack.set_visible_child_name("app");
}
