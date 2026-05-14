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
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::request::{Request, ResetStats, UnlockAllAchievements};
use gtk::gio::{SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{GridView, Label, MenuButton, glib};

pub fn create_bulk_actions(
    application: &MainApplication,
    grid_view: &GridView,
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
                set_app_action_enabled(
                    &application,
                    "export_selected_progress",
                    has_selection,
                );
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

            set_app_action_enabled(&application, "unlock_all_apps", false);
            set_app_action_enabled(&application, "lock_all_apps", false);
            set_app_action_enabled(&application, "export_selected_progress", false);
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            let total_apps = apps_to_unlock.len();
            let progress_label_weak = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let info_label_weak =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            let handle = spawn_blocking(move || {
                let mut failed_apps = Vec::new();
                for (i, (app_id, app_name)) in apps_to_unlock.into_iter().enumerate() {
                    let current_step: u32 = (i as u32) + 1;
                    crate::dev_println!(
                        "[CLIENT] Unlocking app {app_id} ({current_step}/{total_apps})"
                    );

                    let progress_label_weak = progress_label_weak.clone();
                    let info_label_weak = info_label_weak.clone();
                    let app_name_for_label = app_name.clone();
                    MainContext::default().invoke(move || {
                        if let Some(label) = progress_label_weak.upgrade() {
                            label.set_text(&format!("Unlocking {}/{}", current_step, total_apps));
                        }
                        if let Some(label) = info_label_weak.upgrade() {
                            label.set_text(&app_name_for_label);
                        }
                    });

                    let res = UnlockAllAchievements { app_id }.request();

                    if let Err(e) = res {
                        eprintln!("[CLIENT] Error unlocking app {}: {}", app_id, e);
                        failed_apps.push(app_name);
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
                context_menu_button_loading,
                #[weak]
                context_menu_button,
                async move {
                    let failed_apps = handle
                        .await
                        .expect("[CLIENT] Failed to wait for unlock thread to finish");

                    if !failed_apps.is_empty() {
                        let total_failed = failed_apps.len();
                        let display_text = if total_failed > 10 {
                            let first_ten = failed_apps[..10].join("\n");
                            let remaining = total_failed - 10;
                            format!("{}\n\n... and {} more", first_ten, remaining)
                        } else {
                            failed_apps.join("\n")
                        };

                        let dialog = gtk::AlertDialog::builder()
                            .modal(true)
                            .message("Unlock Incomplete")
                            .detail(format!(
                                "Failed to unlock achievements for the following apps:\n\n{}",
                                display_text
                            ))
                            .buttons(["OK"])
                            .build();

                        let parent = application.active_window();
                        let _ = dialog.choose_future(parent.as_ref()).await;
                    }

                    set_app_action_enabled(&application, "unlock_all_apps", true);
                    set_app_action_enabled(&application, "lock_all_apps", true);
                    set_app_action_enabled(&application, "export_selected_progress", true);
                    context_menu_button_loading.set_visible(false);
                    context_menu_button.set_visible(true);
                    grid_view.set_sensitive(true);
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

            set_app_action_enabled(&application, "unlock_all_apps", false);
            set_app_action_enabled(&application, "lock_all_apps", false);
            set_app_action_enabled(&application, "export_selected_progress", false);
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            let total_apps = apps_to_lock.len();
            let progress_label_weak = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let info_label_weak =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            let handle = spawn_blocking(move || {
                for (i, (app_id, app_name)) in apps_to_lock.into_iter().enumerate() {
                    let current_step: u32 = (i as u32) + 1;
                    crate::dev_println!(
                        "[CLIENT] Locking app {app_id} ({current_step}/{total_apps})"
                    );

                    let progress_label_weak = progress_label_weak.clone();
                    let info_label_weak = info_label_weak.clone();
                    let app_name_for_label = app_name.clone();
                    MainContext::default().invoke(move || {
                        if let Some(label) = progress_label_weak.upgrade() {
                            label.set_text(&format!("Locking {}/{}", current_step, total_apps));
                        }
                        if let Some(label) = info_label_weak.upgrade() {
                            label.set_text(&app_name_for_label);
                        }
                    });

                    let res = ResetStats {
                        app_id,
                        achievements_too: true,
                    }
                    .request();

                    if let Err(e) = res {
                        eprintln!("[CLIENT] Error locking app {}: {}", app_id, e);
                        return Err(e);
                    }
                }
                Ok(())
            });

            MainContext::default().spawn_local(clone!(
                #[weak]
                grid_view,
                #[weak]
                application,
                #[weak]
                context_menu_button_loading,
                #[weak]
                context_menu_button,
                async move {
                    let _ = handle.await;
                    set_app_action_enabled(&application, "unlock_all_apps", true);
                    set_app_action_enabled(&application, "lock_all_apps", true);
                    set_app_action_enabled(&application, "export_selected_progress", true);
                    context_menu_button_loading.set_visible(false);
                    context_menu_button.set_visible(true);
                    grid_view.set_sensitive(true);
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
