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
use crate::gui_frontend::application_actions::set_bulk_actions_enabled;
use crate::gui_frontend::dialogs::show_list_dialog;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::request::{ExportApps, ImportApps, Request};
use crate::utils::export_file::{ExportFile, FORMAT_VERSION, iso8601_utc_now};
use crate::utils::ipc_types::AppExport;
use gtk::gio::{ListStore, SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{GridView, Label, MenuButton, glib};
use std::collections::HashMap;
use std::collections::HashSet;

fn has_protected_fields(export: &AppExport) -> bool {
    export.achievements.iter().any(|a| a.permission != 0)
        || export.stats.iter().any(|s| (s.permission & 2) != 0)
}

async fn show_alert(app: Option<&MainApplication>, message: &str, detail: &str) {
    let dlg = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .detail(detail)
        .buttons(["OK"])
        .build();
    let parent = app.and_then(|a| a.active_window());
    let _ = dlg.choose_future(parent.as_ref()).await;
}

pub fn create_progress_actions(
    application: &MainApplication,
    grid_view: &GridView,
    list_store: &ListStore,
    achievement_loader: AchievementLoader,
    context_menu_button: &MenuButton,
    context_menu_button_loading: &MenuButton,
    context_menu_button_loading_progress_label: &Label,
    context_menu_button_info_label: &Label,
) -> (SimpleAction, SimpleAction) {
    let action_export_selected = SimpleAction::new("export_selected_progress", None);
    action_export_selected.set_enabled(false);
    action_export_selected.connect_activate(clone!(
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

            let mut apps: Vec<(u32, String)> = Vec::new();
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
                        apps.push((item.app_id(), item.app_name().to_string()));
                    }
                }
            }
            if apps.is_empty() {
                return;
            }

            let default_name = glib::DateTime::now_local()
                .ok()
                .and_then(|d| d.format("%Y%m%d-%H%M%S").ok())
                .map(|s| format!("samrewritten_progress_{}.json", s))
                .unwrap_or_else(|| "samrewritten_progress.json".to_string());

            let json_filter = gtk::FileFilter::new();
            json_filter.add_pattern("*.json");
            json_filter.set_name(Some("JSON files"));
            let filters = ListStore::new::<gtk::FileFilter>();
            filters.append(&json_filter);

            let dialog = gtk::FileDialog::builder()
                .modal(true)
                .initial_name(&default_name)
                .filters(&filters)
                .default_filter(&json_filter)
                .title("Export selected apps progress")
                .build();

            let parent_window = application.active_window();
            let weak_app = application.downgrade();
            let weak_grid = grid_view.downgrade();
            let weak_btn = context_menu_button.downgrade();
            let weak_btn_loading = context_menu_button_loading.downgrade();
            let weak_progress = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let weak_info =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            MainContext::default().spawn_local(async move {
                let file = match dialog.save_future(parent_window.as_ref()).await {
                    Ok(f) => f,
                    Err(_) => return,
                };
                let Some(path) = file.path() else {
                    return;
                };

                if let Some(app) = weak_app.upgrade() {
                    set_bulk_actions_enabled(&app, false);
                }
                if let Some(grid) = weak_grid.upgrade() {
                    grid.set_sensitive(false);
                }
                if let Some(btn) = weak_btn.upgrade() {
                    btn.set_visible(false);
                }
                if let Some(loading) = weak_btn_loading.upgrade() {
                    loading.set_visible(true);
                }

                let total = apps.len();
                let path_for_task = path.clone();
                let weak_progress_for_thread = weak_progress.clone();
                MainContext::default().invoke(move || {
                    if let Some(label) = weak_progress.upgrade() {
                        label.set_text(&format!("Exporting 0 / {} app(s)…", total));
                    }
                    if let Some(label) = weak_info.upgrade() {
                        label.set_text("");
                    }
                });
                let handle = spawn_blocking(move || {
                    let names: HashMap<u32, String> = apps.iter().cloned().collect();
                    let app_ids: Vec<u32> = apps.into_iter().map(|(id, _)| id).collect();
                    let mut last_done = 0usize;
                    let results =
                        match (ExportApps { app_ids }).request_with_progress(|done, total| {
                            if done == last_done {
                                return;
                            }
                            last_done = done;
                            let label = weak_progress_for_thread.clone();
                            MainContext::default().invoke(move || {
                                if let Some(l) = label.upgrade() {
                                    l.set_text(&format!("Exporting {done} / {total} app(s)…"));
                                }
                            });
                        }) {
                            Ok(results) => results,
                            Err(e) => return Err(format!("Export failed: {e}")),
                        };

                    let mut exports: Vec<AppExport> = Vec::new();
                    let mut failed: Vec<String> = Vec::new();
                    for (app_id, res) in results {
                        let name = names
                            .get(&app_id)
                            .cloned()
                            .unwrap_or_else(|| format!("App {app_id}"));
                        match res {
                            Ok(mut export) => {
                                export.app_name = name;
                                exports.push(export);
                            }
                            Err(e) => {
                                eprintln!("[CLIENT] Export failed for {app_id}: {e}");
                                failed.push(name);
                            }
                        }
                    }

                    let file_struct = ExportFile {
                        format_version: FORMAT_VERSION,
                        exported_at: iso8601_utc_now(),
                        apps: exports,
                    };

                    match serde_json::to_string_pretty(&file_struct) {
                        Ok(content) => match std::fs::write(&path_for_task, content) {
                            Ok(_) => Ok(failed),
                            Err(e) => Err(format!("Failed to write file: {e}")),
                        },
                        Err(e) => Err(format!("Failed to serialize: {e}")),
                    }
                });

                let result = handle.await.expect("[CLIENT] Failed to wait for export");

                if let Some(app) = weak_app.upgrade() {
                    set_bulk_actions_enabled(&app, true);
                }
                if let Some(grid) = weak_grid.upgrade() {
                    grid.set_sensitive(true);
                }
                if let Some(btn) = weak_btn.upgrade() {
                    btn.set_visible(true);
                }
                if let Some(loading) = weak_btn_loading.upgrade() {
                    loading.set_visible(false);
                }

                let app = weak_app.upgrade();
                let parent = app.as_ref().and_then(|a| a.active_window());
                match result {
                    Ok(failed) if failed.is_empty() => {
                        show_alert(
                            app.as_ref(),
                            "Export complete",
                            &format!("Exported {} app(s) to {}", total, path.display()),
                        )
                        .await;
                    }
                    Ok(failed) => {
                        if let Some(parent) = parent {
                            show_list_dialog(
                                &parent,
                                "Export partially complete",
                                &format!(
                                    "Wrote {}\n\nFailed to read data for these apps:",
                                    path.display()
                                ),
                                &failed.join("\n"),
                            );
                        }
                    }
                    Err(e) => {
                        show_alert(app.as_ref(), "Export failed", &e).await;
                    }
                }
            });
        }
    ));

    let action_import_progress = SimpleAction::new("import_progress", None);
    action_import_progress.connect_activate(clone!(
        #[weak]
        grid_view,
        #[weak]
        list_store,
        #[strong]
        achievement_loader,
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
            let json_filter = gtk::FileFilter::new();
            json_filter.add_pattern("*.json");
            json_filter.set_name(Some("JSON files"));
            let filters = ListStore::new::<gtk::FileFilter>();
            filters.append(&json_filter);

            let dialog = gtk::FileDialog::builder()
                .modal(true)
                .filters(&filters)
                .default_filter(&json_filter)
                .title("Import progress")
                .build();

            let parent_window = application.active_window();
            let weak_app = application.downgrade();
            let weak_grid = grid_view.downgrade();
            let weak_btn = context_menu_button.downgrade();
            let weak_btn_loading = context_menu_button_loading.downgrade();
            let weak_progress = glib::object::SendWeakRef::from(
                context_menu_button_loading_progress_label.downgrade(),
            );
            let weak_info =
                glib::object::SendWeakRef::from(context_menu_button_info_label.downgrade());

            let mut library_ids: HashSet<u32> = HashSet::new();
            for i in 0..list_store.n_items() {
                if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>() {
                    library_ids.insert(app.app_id());
                }
            }

            let achievement_loader = achievement_loader.clone();
            MainContext::default().spawn_local(async move {
                let file = match dialog.open_future(parent_window.as_ref()).await {
                    Ok(f) => f,
                    Err(_) => return,
                };
                let Some(path) = file.path() else {
                    return;
                };

                let contents = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        show_alert(
                            weak_app.upgrade().as_ref(),
                            "Import failed",
                            &format!("Could not read file: {e}"),
                        )
                        .await;
                        return;
                    }
                };

                let parsed: ExportFile = match serde_json::from_str(&contents) {
                    Ok(p) => p,
                    Err(e) => {
                        show_alert(
                            weak_app.upgrade().as_ref(),
                            "Import failed",
                            &format!("Could not parse file: {e}"),
                        )
                        .await;
                        return;
                    }
                };
                if parsed.format_version != FORMAT_VERSION {
                    show_alert(
                        weak_app.upgrade().as_ref(),
                        "Import failed",
                        &format!(
                            "Unsupported format version: {} (this build expects {})",
                            parsed.format_version, FORMAT_VERSION
                        ),
                    )
                    .await;
                    return;
                }

                let mut missing: Vec<String> = Vec::new();
                let mut present: Vec<AppExport> = Vec::new();
                for app in parsed.apps {
                    if library_ids.contains(&app.app_id) {
                        present.push(app);
                    } else {
                        let name = if app.app_name.is_empty() {
                            format!("App {}", app.app_id)
                        } else {
                            format!("{} ({})", app.app_name, app.app_id)
                        };
                        missing.push(name);
                    }
                }

                if present.is_empty() {
                    show_alert(
                        weak_app.upgrade().as_ref(),
                        "Nothing to import",
                        "None of the apps in the file are in your current library.",
                    )
                    .await;
                    return;
                }

                let mut protected_apps: Vec<String> = Vec::new();
                for app in &present {
                    if has_protected_fields(app) {
                        let label = if app.app_name.is_empty() {
                            format!("App {}", app.app_id)
                        } else {
                            format!("{} ({})", app.app_name, app.app_id)
                        };
                        protected_apps.push(label);
                    }
                }

                if !protected_apps.is_empty() {
                    let listing = if protected_apps.len() > 10 {
                        format!(
                            "{}\n... and {} more",
                            protected_apps[..10].join("\n"),
                            protected_apps.len() - 10
                        )
                    } else {
                        protected_apps.join("\n")
                    };

                    let dlg = gtk::AlertDialog::builder()
                        .modal(true)
                        .message("Some apps contain protected fields.")
                        .detail(format!(
                            "The following apps contain fields that may not be \
                             importable. Proceed at your own risk.\n\n{}",
                            listing
                        ))
                        .buttons(["Cancel", "Skip these apps", "Proceed anyway"])
                        .cancel_button(0)
                        .default_button(2)
                        .build();
                    let parent = weak_app.upgrade().and_then(|a| a.active_window());
                    let choice = dlg.choose_future(parent.as_ref()).await;
                    match choice {
                        Ok(2) => {} // Proceed: keep all apps; backend skips protected fields
                        Ok(1) => {
                            let protected_ids: HashSet<u32> = present
                                .iter()
                                .filter(|a| has_protected_fields(a))
                                .map(|a| a.app_id)
                                .collect();
                            present.retain(|a| !protected_ids.contains(&a.app_id));
                            if present.is_empty() {
                                show_alert(
                                    weak_app.upgrade().as_ref(),
                                    "Nothing to import",
                                    "All remaining apps were skipped.",
                                )
                                .await;
                                return;
                            }
                        }
                        _ => return,
                    }
                }

                if let Some(app) = weak_app.upgrade() {
                    set_bulk_actions_enabled(&app, false);
                }
                if let Some(grid) = weak_grid.upgrade() {
                    grid.set_sensitive(false);
                }
                if let Some(btn) = weak_btn.upgrade() {
                    btn.set_visible(false);
                }
                if let Some(loading) = weak_btn_loading.upgrade() {
                    loading.set_visible(true);
                }

                let total = present.len();
                let weak_progress_for_thread = weak_progress.clone();
                MainContext::default().invoke(move || {
                    if let Some(label) = weak_progress.upgrade() {
                        label.set_text(&format!("Importing 0 / {} app(s)…", total));
                    }
                    if let Some(label) = weak_info.upgrade() {
                        label.set_text("");
                    }
                });
                let affected_ids: Vec<u32> = present.iter().map(|a| a.app_id).collect();
                let names_by_id: HashMap<u32, String> = present
                    .iter()
                    .map(|a| {
                        let label = if a.app_name.is_empty() {
                            format!("App {}", a.app_id)
                        } else {
                            a.app_name.clone()
                        };
                        (a.app_id, label)
                    })
                    .collect();
                let handle = spawn_blocking(move || {
                    let mut last_done = 0usize;
                    let results =
                        match (ImportApps { apps: present }).request_with_progress(|done, total| {
                            if done == last_done {
                                return;
                            }
                            last_done = done;
                            let label = weak_progress_for_thread.clone();
                            MainContext::default().invoke(move || {
                                if let Some(l) = label.upgrade() {
                                    l.set_text(&format!("Importing {done} / {total} app(s)…"));
                                }
                            });
                        }) {
                            Ok(results) => results,
                            Err(e) => {
                                return (
                                    0,
                                    0,
                                    0,
                                    0,
                                    vec![format!("Import failed: {e}")],
                                    Vec::new(),
                                );
                            }
                        };

                    let mut total_ach: usize = 0;
                    let mut total_stat: usize = 0;
                    let mut total_skipped_protected: usize = 0;
                    let mut total_skipped_unwriteable: usize = 0;
                    let mut errors: Vec<String> = Vec::new();
                    let mut reset_candidates: Vec<String> = Vec::new();
                    for (app_id, res) in results {
                        let label = names_by_id
                            .get(&app_id)
                            .cloned()
                            .unwrap_or_else(|| format!("App {}", app_id));
                        match res {
                            Ok(summary) => {
                                total_ach += summary.achievements_applied;
                                total_stat += summary.stats_applied;
                                total_skipped_protected += summary.skipped_protected.len();
                                total_skipped_unwriteable += summary.skipped_unwriteable.len();
                                for err in summary.errors {
                                    errors.push(format!("{}: {}", label, err));
                                }
                                if summary.reset_would_help {
                                    reset_candidates.push(label);
                                }
                            }
                            Err(e) => {
                                errors.push(format!("{}: {}", label, e));
                            }
                        }
                    }
                    (
                        total_ach,
                        total_stat,
                        total_skipped_protected,
                        total_skipped_unwriteable,
                        errors,
                        reset_candidates,
                    )
                });

                let (
                    applied_ach,
                    applied_stat,
                    skipped_protected,
                    skipped_unwriteable,
                    errors,
                    reset_candidates,
                ) = handle.await.expect("[CLIENT] Failed to wait for import");

                if let Some(app) = weak_app.upgrade() {
                    set_bulk_actions_enabled(&app, true);
                }
                if let Some(grid) = weak_grid.upgrade() {
                    grid.set_sensitive(true);
                }
                if let Some(btn) = weak_btn.upgrade() {
                    btn.set_visible(true);
                }
                if let Some(loading) = weak_btn_loading.upgrade() {
                    loading.set_visible(false);
                }

                let mut intro = format!(
                    "Applied {} achievement(s) and {} stat(s).",
                    applied_ach, applied_stat
                );
                if skipped_protected > 0 {
                    intro.push_str(&format!(
                        "\nSkipped {} protected field(s).",
                        skipped_protected
                    ));
                }
                if skipped_unwriteable > 0 {
                    intro.push_str(&format!(
                        "\nSkipped {} unwriteable stat(s) (out of range or increment-only).",
                        skipped_unwriteable
                    ));
                }

                let mut sections: Vec<String> = Vec::new();
                if !reset_candidates.is_empty() {
                    sections.push(format!(
                        "Would succeed if you reset stats first:\n{}",
                        reset_candidates.join("\n")
                    ));
                }
                if !missing.is_empty() {
                    sections.push(format!(
                        "Skipped (not in your library):\n{}",
                        missing.join("\n")
                    ));
                }
                if !errors.is_empty() {
                    sections.push(format!("Errors:\n{}", errors.join("\n")));
                }

                let app = weak_app.upgrade();
                if sections.is_empty() {
                    show_alert(app.as_ref(), "Import complete", &intro).await;
                } else if let Some(parent) = app.as_ref().and_then(|a| a.active_window()) {
                    show_list_dialog(&parent, "Import complete", &intro, &sections.join("\n\n"));
                }

                for id in affected_ids {
                    achievement_loader.refresh_app(id, &list_store);
                }
            });
        }
    ));

    (action_export_selected, action_import_progress)
}
