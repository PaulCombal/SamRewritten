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

use super::PrefetchedProgress;
use super::achievement_loader::AchievementLoader;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::application_actions::{
    set_app_action_enabled, set_timed_unlock_actions_enabled,
};
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::stat::GStatObject;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{
    AppProgress, GetAchievementsAndStats, GetRunningApps, GetSubscribedAppList, Request, ResetStats,
};
use crate::gui_frontend::ui_components::set_achievement_languages;
use crate::utils::action_journal::{self, Batch, Change, Op};
use crate::utils::format::format_achievement_progress;
use crate::utils::ipc_types::SamError;
use gtk::gio::{ListStore, Settings, SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{GridView, Label, ScrolledWindow, SearchEntry, Stack, glib};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[allow(clippy::too_many_arguments)]
pub fn create_refresh_app_list_action(
    application: &MainApplication,
    grid_view: &GridView,
    list_store: &ListStore,
    list_scrolled_window: &ScrolledWindow,
    list_of_apps_or_no_result: &Stack,
    app_list_no_result_label: &Label,
    list_stack: &Stack,
    search_entry: &SearchEntry,
    idle_count: Rc<Cell<usize>>,
    achievement_loader: AchievementLoader,
    on_library_loaded: Rc<dyn Fn()>,
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
        #[strong]
        idle_count,
        #[strong]
        achievement_loader,
        #[strong]
        on_library_loaded,
        move |_, _| {
            list_stack.set_visible_child_name("loading");
            set_app_action_enabled(&application, "unlock_all_apps", false);
            set_app_action_enabled(&application, "lock_all_apps", false);
            search_entry.set_sensitive(false);
            let apps = spawn_blocking(move || {
                GetSubscribedAppList {
                    include_playtime: true,
                    with_achievement_counts: false,
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
                #[strong]
                idle_count,
                #[strong]
                achievement_loader,
                #[strong]
                on_library_loaded,
                async move {
                    match apps.await {
                        Ok(Ok(app_vec)) => {
                            search_entry.set_sensitive(true);

                            if app_vec.is_empty() {
                                app_list_no_result_label.set_text(tr("No apps found on your account. Search for App Id to get started.").as_str());
                                list_of_apps_or_no_result.set_visible_child_name("empty");
                                list_scrolled_window.set_child(Some(&grid_view));
                                list_stack.set_visible_child_name("list");
                            } else {
                                list_store.remove_all();
                                idle_count.set(0);
                                GSteamAppObject::rebuild_local_banner_index();
                                let models: Vec<GSteamAppObject> =
                                    app_vec.into_iter().map(GSteamAppObject::new).collect();
                                // Drop any prior loader state; cards will
                                // re-queue themselves via `prioritize()` from
                                // the cell-bind callback as they scroll in.
                                achievement_loader.reset();
                                list_store.extend_from_slice(&models);
                                list_scrolled_window.set_child(Some(&grid_view));
                                list_stack.set_visible_child_name("list");
                                app_list_no_result_label.set_text(tr("No results. Check for spelling mistakes or try typing an App Id.").as_str());
                                on_library_loaded();

                                // Sync idle state from the orchestrator: any app it's
                                // currently holding open should show as idling in the UI.
                                let running = spawn_blocking(|| GetRunningApps.request());
                                MainContext::default().spawn_local(clone!(
                                    #[weak]
                                    list_store,
                                    #[strong]
                                    idle_count,
                                    async move {
                                        let Ok(Ok(running)) = running.await else {
                                            return;
                                        };
                                        let running: std::collections::HashSet<u32> =
                                            running.into_iter().collect();
                                        let n = list_store.n_items();
                                        for i in 0..n {
                                            if let Some(item) = list_store.item(i)
                                                && let Ok(app) = item.downcast::<GSteamAppObject>()
                                                && running.contains(&app.app_id())
                                            {
                                                app.set_is_idling(true);
                                            }
                                        }
                                        super::recompute_idle_cap(&list_store, &idle_count);
                                    }
                                ));
                            }
                        },
                        Ok(Err(SamError::AppListRetrievalFailed)) => {
                            search_entry.set_sensitive(true);
                            app_list_no_result_label.set_text(tr("Failed to load library. Check your internet connection. Search for App Id to get started.").as_str());
                            list_of_apps_or_no_result.set_visible_child_name("empty");
                            list_scrolled_window.set_child(Some(&grid_view));
                            list_stack.set_visible_child_name("list");
                        },
                        Ok(Err(sam_error)) => {
                            eprintln!("[CLIENT] Unknown error: {}", sam_error);
                            list_stack.set_visible_child_name("disconnected");
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

pub fn create_rescan_counts_action(
    list_store: &ListStore,
    achievement_loader: &AchievementLoader,
    counts_wanted_by_profile: &Rc<Cell<bool>>,
    sync_counts_state: &Rc<dyn Fn(bool)>,
) -> SimpleAction {
    let action = SimpleAction::new("rescan_achievement_counts", None);
    action.connect_activate(clone!(
        #[weak]
        list_store,
        #[strong]
        achievement_loader,
        #[strong]
        counts_wanted_by_profile,
        #[strong]
        sync_counts_state,
        move |_, _| {
            if achievement_loader.is_rescanning() {
                return;
            }
            // Without this the sweep is cancelled as soon as nothing on screen
            // needs counts, which is the usual case on the app list.
            counts_wanted_by_profile.set(true);
            achievement_loader.rescan_all(&list_store);
            sync_counts_state(false);
        }
    ));
    action
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
    app_achievements_stack: &Stack,
    cancel_timed_unlock: &Arc<AtomicBool>,
    prefetched_progress: &PrefetchedProgress,
    settings: &Settings,
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
        app_achievements_stack,
        #[strong]
        cancel_timed_unlock,
        #[strong]
        prefetched_progress,
        #[strong]
        settings,
        move |_, _| {
            // Before the teardown below, which would leave nothing to rebuild from.
            let Some(app_id_copy) = app_id.get() else {
                return;
            };

            app_stack.set_visible_child_name("loading");
            set_app_action_enabled(&application, "refresh_achievements_list", false);
            app_achievements_model.remove_all();
            app_stat_model.remove_all();
            cancel_timed_unlock.store(true, std::sync::atomic::Ordering::Relaxed);
            app_achievements_stack.set_visible_child_name("manual");

            let language = settings.string("achievement-language").to_string();
            let prefetched = prefetched_progress
                .borrow_mut()
                .take()
                .filter(|(fetched_for, fetched_in, _)| {
                    *fetched_for == app_id_copy && *fetched_in == language
                })
                .map(|(_, _, progress)| progress);
            let requested_language = language.clone();
            let handle = spawn_blocking(move || match prefetched {
                Some(progress) => Ok(progress),
                None => GetAchievementsAndStats {
                    app_id: app_id_copy,
                    launch: false,
                    language,
                }
                .request(),
            });

            MainContext::default().spawn_local(clone!(
                #[strong]
                app_unlocked_achievements_count,
                #[strong]
                settings,
                #[strong]
                app_id,
                async move {
                    let result = handle.await;

                    // Another game may own the models by now.
                    if app_id.get() != Some(app_id_copy) {
                        return;
                    }

                    if let Ok(Ok(AppProgress {
                        achievements,
                        stats,
                        languages,
                    })) = result
                    {
                        set_achievement_languages(&languages);

                        let achievement_len = achievements.len();
                        let stat_len = stats.len();
                        let achievement_unlocked_len =
                            achievements.iter().filter(|ach| ach.is_achieved).count();
                        app_unlocked_achievements_count.set(achievement_unlocked_len);

                        app_stats_count_value.set_label(&format!("{stat_len}"));
                        app_achievement_count_value.set_label(&format_achievement_progress(
                            achievement_unlocked_len,
                            achievement_len,
                        ));

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
                        } else if stat_len > 0 {
                            app_stack.set_visible_child_name("stats");
                        } else {
                            app_stack.set_visible_child_name("empty");
                        }
                    } else {
                        app_stack.set_visible_child_name("failed");
                    }

                    set_timed_unlock_actions_enabled(&application, true);

                    if settings.string("achievement-language") != requested_language {
                        application.activate_action("refresh_achievements_list", None);
                    }
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
    /// What a reset is about to destroy, as changes that would put it back, and
    /// only what actually moves.
    ///
    /// Stats are recorded as landing on zero: Steam restores each to its schema
    /// default, so the history says what a stat *was* rather than claiming to
    /// know what the reset left it at.
    fn wiped_by_reset(achievements: &ListStore, stats: &ListStore) -> Vec<Change> {
        if !action_journal::is_enabled() {
            return Vec::new();
        }
        let mut changes = Vec::new();
        for achievement in achievements
            .into_iter()
            .flatten()
            .filter_map(|o| o.downcast::<GAchievementObject>().ok())
        {
            if achievement.is_achieved() && achievement.permission() == 0 {
                changes.push(Change::Achievement {
                    id: achievement.id(),
                    name: achievement.name(),
                    before: true,
                    after: false,
                });
            }
        }
        for stat in stats
            .into_iter()
            .flatten()
            .filter_map(|o| o.downcast::<GStatObject>().ok())
        {
            let before = stat.original_value();
            if (stat.permission() & 2) != 0 || before == 0.0 {
                continue;
            }
            changes.push(if stat.is_integer() {
                Change::IntStat {
                    id: stat.id(),
                    name: stat.display_name(),
                    before: before as i32,
                    after: 0,
                }
            } else {
                Change::FloatStat {
                    id: stat.id(),
                    name: stat.display_name(),
                    before: before as f32,
                    after: 0.0,
                }
            });
        }
        changes
    }

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
                        .message(tr("Reset Everything").as_str())
                        .detail(tr("This will reset all achievements and stats for this app. Are you sure?").as_str())
                        .buttons([tr("Cancel").as_str(), tr("Sure, reset").as_str()])
                        .cancel_button(0)
                        .default_button(0)
                        .build();

                    let parent = application.active_window();
                    let response = dialog.choose_future(parent.as_ref()).await;

                    if response != Ok(1) {
                        return;
                    }

                    // The dialog was awaited, so the page may have been left since.
                    let Some(app_id_copy) = app_id.get() else {
                        return;
                    };

                    let undo = wiped_by_reset(&app_achievements_model, &app_stat_model);

                    app_stack.set_visible_child_name("loading");
                    set_app_action_enabled(&application, "clear_all_stats_and_achievements", false);
                    app_achievements_model.remove_all();
                    app_stat_model.remove_all();

                    let handle = spawn_blocking(move || {
                        ResetStats {
                            app_id: app_id_copy,
                            achievements_too: true,
                        }
                        .request()
                    });

                    let result = handle.await;
                    if app_id.get() != Some(app_id_copy) {
                        return;
                    }

                    // `Ok(Ok(false))` is Steam taking the reset and failing to
                    // store it, so it belongs with the failures.
                    let Ok(Ok(true)) = result else {
                        set_app_action_enabled(
                            &application,
                            "clear_all_stats_and_achievements",
                            true,
                        );
                        return app_stack.set_visible_child_name("failed");
                    };

                    Batch::new(Op::ResetApp, app_id_copy, "").record(undo);
                    action_refresh_achievements_list.activate(None);
                }
            ));
        }
    ));
    action_clear_all_stats_and_achievements
}
