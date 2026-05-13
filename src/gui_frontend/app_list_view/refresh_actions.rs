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
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::stat::GStatObject;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::request::{
    GetAchievements, GetStats, GetSubscribedAppList, Request, ResetStats,
};
use crate::utils::ipc_types::SamError;
use gtk::gio::{ListStore, SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{Adjustment, Button, GridView, Label, ScrolledWindow, SearchEntry, Stack, glib};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn create_refresh_app_list_action(
    application: &MainApplication,
    grid_view: &GridView,
    list_store: &ListStore,
    list_scrolled_window: &ScrolledWindow,
    list_of_apps_or_no_result: &Stack,
    app_list_no_result_label: &Label,
    list_stack: &Stack,
    search_entry: &SearchEntry,
) -> SimpleAction {
    let action_refresh_app_list = SimpleAction::new("refresh_app_list", None);
    action_refresh_app_list.connect_activate(clone!(
        #[strong]
        grid_view,
        #[strong]
        list_store,
        #[weak]
        list_scrolled_window,
        #[weak]
        list_of_apps_or_no_result,
        #[weak]
        app_list_no_result_label,
        #[weak]
        list_stack,
        #[weak]
        search_entry,
        #[weak]
        application,
        move |_, _| {
            list_stack.set_visible_child_name("loading");
            set_app_action_enabled(&application, "unlock_all_apps", false);
            set_app_action_enabled(&application, "lock_all_apps", false);
            search_entry.set_sensitive(false);
            let apps = spawn_blocking(move || {
                GetSubscribedAppList {
                    include_playtime: true,
                }
                .request()
            });
            MainContext::default().spawn_local(clone!(
                #[weak]
                grid_view,
                #[weak]
                list_scrolled_window,
                #[weak]
                list_of_apps_or_no_result,
                #[weak]
                app_list_no_result_label,
                #[weak]
                list_store,
                #[weak]
                list_stack,
                #[weak]
                search_entry,
                async move {
                    match apps.await {
                        Ok(Ok(app_vec)) => {
                            search_entry.set_sensitive(true);

                            if app_vec.is_empty() {
                                app_list_no_result_label.set_text("No apps found on your account. Search for App Id to get started.");
                                list_of_apps_or_no_result.set_visible_child_name("empty");
                                list_scrolled_window.set_child(Some(&grid_view));
                                list_stack.set_visible_child_name("list");
                            } else {
                                list_store.remove_all();
                                GSteamAppObject::rebuild_local_banner_index();
                                let models: Vec<GSteamAppObject> =
                                    app_vec.into_iter().map(GSteamAppObject::new).collect();
                                list_store.extend_from_slice(&models);
                                list_scrolled_window.set_child(Some(&grid_view));
                                list_stack.set_visible_child_name("list");
                                app_list_no_result_label.set_text("No results. Check for spelling mistakes or try typing an App Id.");
                            }
                        },
                        Ok(Err(SamError::AppListRetrievalFailed)) => {
                            search_entry.set_sensitive(true);
                            app_list_no_result_label.set_text("Failed to load library. Check your internet connection. Search for App Id to get started.");
                            list_of_apps_or_no_result.set_visible_child_name("empty");
                            list_scrolled_window.set_child(Some(&grid_view));
                            list_stack.set_visible_child_name("list");
                        },
                        Ok(Err(sam_error)) => {
                            eprintln!("[CLIENT] Unknown error: {}", sam_error);
                            let label = Label::new(Some("SamRewritten could not connect to Steam. Is it running?"));
                            list_scrolled_window.set_child(Some(&label));
                            list_stack.set_visible_child_name("list");
                        }
                        Err(join_error) => {
                            eprintln!("Spawn blocking error: {:?}", join_error);
                        }
                    };
                }
            ));
        }
    ));
    action_refresh_app_list
}

#[allow(clippy::too_many_arguments)]
pub fn create_refresh_achievements_action(
    application: &MainApplication,
    app_id: &Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: &Rc<Cell<usize>>,
    app_achievements_model: &ListStore,
    app_stat_model: &ListStore,
    app_achievement_count_value: &Label,
    app_stats_count_value: &Label,
    app_stack: &Stack,
    achievements_manual_adjustement: &Adjustment,
    achievements_manual_start: &Button,
    app_achievements_stack: &Stack,
    cancel_timed_unlock: &Arc<AtomicBool>,
) -> SimpleAction {
    let action_refresh_achievements_list = SimpleAction::new("refresh_achievements_list", None);
    action_refresh_achievements_list.set_enabled(false);
    action_refresh_achievements_list.connect_activate(clone!(
        #[strong]
        app_id,
        #[strong]
        app_unlocked_achievements_count,
        #[weak]
        application,
        #[weak]
        app_achievements_model,
        #[weak]
        app_stat_model,
        #[weak]
        app_achievement_count_value,
        #[weak]
        app_stats_count_value,
        #[weak]
        app_stack,
        #[weak]
        achievements_manual_adjustement,
        #[weak]
        achievements_manual_start,
        #[weak]
        app_achievements_stack,
        #[strong]
        cancel_timed_unlock,
        move |_, _| {
            app_stack.set_visible_child_name("loading");
            set_app_action_enabled(&application, "refresh_achievements_list", false);
            app_achievements_model.remove_all();
            app_stat_model.remove_all();
            cancel_timed_unlock.store(true, std::sync::atomic::Ordering::Relaxed);
            app_achievements_stack.set_visible_child_name("manual");

            let app_id_copy = app_id.get().unwrap();
            let handle = spawn_blocking(move || {
                let achievements = GetAchievements {
                    app_id: app_id_copy,
                }
                .request();
                let stats = GetStats {
                    app_id: app_id_copy,
                }
                .request();
                (achievements, stats)
            });

            MainContext::default().spawn_local(clone!(
                #[strong]
                app_unlocked_achievements_count,
                async move {
                    let Ok((Ok(achievements), Ok(stats))) = handle.await else {
                        return app_stack.set_visible_child_name("failed");
                    };

                    let achievement_len = achievements.len();
                    let achievement_unlocked_len =
                        achievements.iter().filter(|ach| ach.is_achieved).count();
                    app_unlocked_achievements_count.set(achievement_unlocked_len);

                    app_stats_count_value.set_label(&format!("{}", stats.len()));
                    app_achievement_count_value
                        .set_label(&format!("{achievement_unlocked_len} / {achievement_len}"));

                    let objects: Vec<GAchievementObject> = achievements
                        .into_iter()
                        .map(GAchievementObject::new)
                        .collect();
                    app_achievements_model.extend_from_slice(&objects);

                    let objects: Vec<GStatObject> =
                        stats.into_iter().map(GStatObject::new).collect();
                    app_stat_model.extend_from_slice(&objects);

                    if achievement_len > 0 {
                        app_stack.set_visible_child_name("achievements");
                    } else {
                        app_stack.set_visible_child_name("empty");
                    }

                    achievements_manual_start
                        .set_sensitive(achievement_unlocked_len != achievement_len);

                    let lower = std::cmp::min(achievement_unlocked_len + 1, achievement_len);
                    achievements_manual_adjustement.set_lower(lower as f64);
                    achievements_manual_adjustement.set_upper(achievement_len as f64);
                    achievements_manual_adjustement.set_value(achievement_len as f64);

                    set_app_action_enabled(&application, "refresh_achievements_list", true);
                    set_app_action_enabled(&application, "clear_all_stats_and_achievements", true);
                }
            ));
        }
    ));
    action_refresh_achievements_list
}

pub fn create_clear_all_action(
    application: &MainApplication,
    app_id: &Rc<Cell<Option<u32>>>,
    app_achievements_model: &ListStore,
    app_stat_model: &ListStore,
    action_refresh_achievements_list: &SimpleAction,
    app_stack: &Stack,
) -> SimpleAction {
    let action_clear_all_stats_and_achievements =
        SimpleAction::new("clear_all_stats_and_achievements", None);
    action_clear_all_stats_and_achievements.set_enabled(false);
    action_clear_all_stats_and_achievements.connect_activate(clone!(
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        app_achievements_model,
        #[weak]
        app_stat_model,
        #[weak]
        action_refresh_achievements_list,
        #[weak]
        app_stack,
        move |_, _| {
            MainContext::default().spawn_local(clone!(
                #[strong]
                app_id,
                #[strong]
                application,
                #[strong]
                app_achievements_model,
                #[strong]
                app_stat_model,
                #[strong]
                action_refresh_achievements_list,
                #[strong]
                app_stack,
                async move {
                    let dialog = gtk::AlertDialog::builder()
                        .modal(true)
                        .message("Reset Everything")
                        .detail("This will reset all achievements and stats for this app. Are you sure?")
                        .buttons(["Cancel", "Sure, reset"])
                        .cancel_button(0)
                        .default_button(0)
                        .build();

                    let parent = application.active_window();
                    let response = dialog.choose_future(parent.as_ref()).await;

                    if response != Ok(1) {
                        return;
                    }

                    app_stack.set_visible_child_name("loading");
                    set_app_action_enabled(&application, "clear_all_stats_and_achievements", false);
                    app_achievements_model.remove_all();
                    app_stat_model.remove_all();

                    let app_id_copy = app_id.get().unwrap();
                    let handle = spawn_blocking(move || {
                        ResetStats {
                            app_id: app_id_copy,
                            achievements_too: true,
                        }
                        .request()
                    });

                    let Ok(Ok(_success)) = handle.await else {
                        return app_stack.set_visible_child_name("failed");
                    };

                    action_refresh_achievements_list.activate(None);
                }
            ));
        }
    ));
    action_clear_all_stats_and_achievements
}
