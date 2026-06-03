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

use super::achievement_loader::AchievementLoader;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::application_actions::{set_app_action_enabled, set_bulk_actions_enabled};
use crate::gui_frontend::dialogs::show_list_dialog;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{Request, ResetApps, UnlockAllApps};
use gtk::gio::{ListStore, SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{GridView, Label, MenuButton, glib};
use std::collections::HashMap;

pub fn create_bulk_actions(
    application: &MainApplication,
    grid_view: &GridView,
    list_store: &ListStore,
    achievement_loader: AchievementLoader,
    context_menu_button: &MenuButton,
    context_menu_button_loading: &MenuButton,
    context_menu_button_loading_progress_label: &Label,
    context_menu_button_info_label: &Label,
) -> (SimpleAction, SimpleAction, SimpleAction, SimpleAction) {
    let action_select_all_apps = SimpleAction::new("select_all_apps", None);
    action_select_all_apps.connect_activate(clone!(
        #[weak]
        grid_view,
        #[weak]
        application,
        move |_, _| {
            if let Some(selection_model) = grid_view.model() {
                selection_model.select_all();
                let has_selection = !selection_model.selection().is_empty();
                set_app_action_enabled(&application, "unlock_all_apps", has_selection);
                set_app_action_enabled(&application, "lock_all_apps", has_selection);
                set_app_action_enabled(&application, "export_selected_progress", has_selection);
            }
        }
    ));

    let action_unselect_all_apps = SimpleAction::new("unselect_all_apps", None);
    action_unselect_all_apps.connect_activate(clone!(
        #[weak]
        grid_view,
        #[weak]
        application,
        move |_, _| {
            if let Some(selection_model) = grid_view.model() {
                selection_model.unselect_all();
                set_app_action_enabled(&application, "unlock_all_apps", false);
                set_app_action_enabled(&application, "lock_all_apps", false);
                set_app_action_enabled(&application, "export_selected_progress", false);
            }
        }
    ));

    let action_unlock_all_selected = SimpleAction::new("unlock_all_apps", None);
    action_unlock_all_selected.set_enabled(false);
    action_unlock_all_selected.connect_activate(clone!(
        #[weak]
        grid_view,
        #[weak]
        application,
        #[weak]
        list_store,
        #[strong]
        achievement_loader,
        #[weak]
        context_menu_button,
        #[weak]
        context_menu_button_loading,
        #[weak]
        context_menu_button_loading_progress_label,
        #[weak]
        context_menu_button_info_label,
        move |_, _| {
            let Some(selection_model) = grid_view.model() else {
                return;
            };
            let selection = selection_model.selection();

            let mut apps_to_unlock = std::collections::HashMap::new();

            if let Some((mut iter, first)) = gtk::BitsetIter::init_first(&selection) {
                let mut indices = vec![first];
                for idx in iter.by_ref() {
                    indices.push(idx);
                }

                for index in indices {
                    if let Some(item) = selection_model
                        .item(index)
                        .and_downcast::<GSteamAppObject>()
                    {
                        apps_to_unlock.insert(item.app_id(), item.app_name());
                    }
                }
            }

            if apps_to_unlock.is_empty() {
                return;
            }

            set_bulk_actions_enabled(&application, false);
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            let total_apps = apps_to_unlock.len();
            let affected_ids: Vec<u32> = apps_to_unlock.keys().copied().collect();
            let progress_label_weak = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let info_label_weak =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            let progress_label_for_thread = progress_label_weak.clone();
            MainContext::default().invoke(move || {
                if let Some(label) = progress_label_weak.upgrade() {
                    label.set_text(
                        &tr("Unlocking {done} / {total} app(s)…")
                            .replace("{done}", "0")
                            .replace("{total}", &total_apps.to_string()),
                    );
                }
                if let Some(label) = info_label_weak.upgrade() {
                    label.set_text("");
                }
            });

            let handle = spawn_blocking(move || {
                let names: HashMap<u32, String> = apps_to_unlock.clone();
                let app_ids: Vec<u32> = apps_to_unlock.into_keys().collect();
                let mut last_done = 0usize;
                let results =
                    match (UnlockAllApps { app_ids }).request_with_progress(|done, total| {
                        if done == last_done {
                            return;
                        }
                        last_done = done;
                        let label = progress_label_for_thread.clone();
                        MainContext::default().invoke(move || {
                            if let Some(l) = label.upgrade() {
                                l.set_text(
                                    &tr("Unlocking {done} / {total} app(s)…")
                                        .replace("{done}", &done.to_string())
                                        .replace("{total}", &total.to_string()),
                                );
                            }
                        });
                    }) {
                        Ok(results) => results,
                        Err(e) => {
                            eprintln!("[CLIENT] Bulk unlock failed: {e}");
                            return names.into_values().collect::<Vec<_>>();
                        }
                    };

                let mut failed_apps = Vec::new();
                for (app_id, res) in results {
                    if let Err(e) = res {
                        eprintln!("[CLIENT] Error unlocking app {}: {}", app_id, e);
                        if let Some(name) = names.get(&app_id) {
                            failed_apps.push(name.clone());
                        }
                    }
                }

                failed_apps
            });

            MainContext::default().spawn_local(clone!(
                #[weak]
                grid_view,
                #[weak]
                application,
                #[weak]
                list_store,
                #[strong]
                achievement_loader,
                #[weak]
                context_menu_button_loading,
                #[weak]
                context_menu_button,
                async move {
                    let failed_apps = handle
                        .await
                        .expect("[CLIENT] Failed to wait for unlock thread to finish");

                    if !failed_apps.is_empty()
                        && let Some(parent) = application.active_window()
                    {
                        show_list_dialog(
                            &parent,
                            tr("Unlock Incomplete").as_str(),
                            tr("Failed to unlock achievements for the following apps:").as_str(),
                            &failed_apps.join("\n"),
                        );
                    }

                    set_bulk_actions_enabled(&application, true);
                    context_menu_button_loading.set_visible(false);
                    context_menu_button.set_visible(true);
                    grid_view.set_sensitive(true);

                    for id in affected_ids {
                        achievement_loader.refresh_app(id, &list_store);
                    }
                }
            ));
        }
    ));

    let action_lock_all_selected = SimpleAction::new("lock_all_apps", None);
    action_lock_all_selected.set_enabled(false);
    action_lock_all_selected.connect_activate(clone!(
        #[weak]
        grid_view,
        #[weak]
        application,
        #[weak]
        list_store,
        #[strong]
        achievement_loader,
        #[weak]
        context_menu_button,
        #[weak]
        context_menu_button_loading,
        #[weak]
        context_menu_button_loading_progress_label,
        #[weak]
        context_menu_button_info_label,
        move |_, _| {
            let Some(selection_model) = grid_view.model() else {
                return;
            };
            let selection = selection_model.selection();

            let mut apps_to_lock = std::collections::HashMap::new();

            if let Some((mut iter, first)) = gtk::BitsetIter::init_first(&selection) {
                let mut indices = vec![first];
                for idx in iter.by_ref() {
                    indices.push(idx);
                }

                for index in indices {
                    if let Some(item) = selection_model
                        .item(index)
                        .and_downcast::<GSteamAppObject>()
                    {
                        apps_to_lock.insert(item.app_id(), item.app_name());
                    }
                }
            }

            if apps_to_lock.is_empty() {
                return;
            }

            set_bulk_actions_enabled(&application, false);
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            let total_apps = apps_to_lock.len();
            let affected_ids: Vec<u32> = apps_to_lock.keys().copied().collect();
            let progress_label_weak = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let info_label_weak =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            let progress_label_for_thread = progress_label_weak.clone();
            MainContext::default().invoke(move || {
                if let Some(label) = progress_label_weak.upgrade() {
                    label.set_text(
                        &tr("Locking {done} / {total} app(s)…")
                            .replace("{done}", "0")
                            .replace("{total}", &total_apps.to_string()),
                    );
                }
                if let Some(label) = info_label_weak.upgrade() {
                    label.set_text("");
                }
            });

            let handle = spawn_blocking(move || {
                let app_ids: Vec<u32> = apps_to_lock.into_keys().collect();
                let mut last_done = 0usize;
                match (ResetApps {
                    app_ids,
                    achievements_too: true,
                })
                .request_with_progress(|done, total| {
                    if done == last_done {
                        return;
                    }
                    last_done = done;
                    let label = progress_label_for_thread.clone();
                    MainContext::default().invoke(move || {
                        if let Some(l) = label.upgrade() {
                            l.set_text(
                                &tr("Locking {done} / {total} app(s)…")
                                    .replace("{done}", &done.to_string())
                                    .replace("{total}", &total.to_string()),
                            );
                        }
                    });
                }) {
                    Ok(results) => {
                        for (app_id, res) in results {
                            if let Err(e) = res {
                                eprintln!("[CLIENT] Error locking app {}: {}", app_id, e);
                            }
                        }
                    }
                    Err(e) => eprintln!("[CLIENT] Bulk lock failed: {e}"),
                }
            });

            MainContext::default().spawn_local(clone!(
                #[weak]
                grid_view,
                #[weak]
                application,
                #[weak]
                list_store,
                #[strong]
                achievement_loader,
                #[weak]
                context_menu_button_loading,
                #[weak]
                context_menu_button,
                async move {
                    let _ = handle.await;
                    set_bulk_actions_enabled(&application, true);
                    context_menu_button_loading.set_visible(false);
                    context_menu_button.set_visible(true);
                    grid_view.set_sensitive(true);

                    for id in affected_ids {
                        achievement_loader.refresh_app(id, &list_store);
                    }
                }
            ));
        }
    ));

    (
        action_select_all_apps,
        action_unselect_all_apps,
        action_unlock_all_selected,
        action_lock_all_selected,
    )
}
