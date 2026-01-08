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

use crate::backend::app_lister::{AppModel, AppModelType};
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::app_list_view_callbacks::switch_from_app_list_to_app;
use crate::gui_frontend::app_view::create_app_view;
use crate::gui_frontend::application_actions::{set_app_action_enabled, setup_app_actions};
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::stat::GStatObject;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::request::{
    GetAchievements, GetStats, GetSubscribedAppList, Request, ResetStats, StopApp,
    UnlockAllAchievements,
};
use crate::gui_frontend::ui_components::{
    create_about_dialog, create_context_menu_button, set_context_popover_to_app_list_context,
};
use crate::gui_frontend::widgets::steam_app_card::SteamAppCard;
use crate::utils::app_paths::get_executable_path;
use crate::utils::arguments::parse_gui_arguments;
use crate::utils::ipc_types::SamError;
use gtk::gio::{ApplicationCommandLine, ListStore, SimpleAction, spawn_blocking};
use gtk::glib::translate::FromGlib;
use gtk::glib::{ExitCode, SignalHandlerId};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box, Button, FilterListModel, GridView, HeaderBar, Image, Label,
    ListItem, MultiSelection, Orientation, PolicyType, ScrolledWindow, SearchEntry,
    SignalListItemFactory, Spinner, Stack, StackTransitionType, Widget,
};
use gtk::{IconSize, glib};
use std::cell::Cell;
use std::ffi::c_ulong;
use std::process::Command;
use std::rc::Rc;

pub fn create_main_ui(
    application: &MainApplication,
    cmd_line: &ApplicationCommandLine,
) -> ExitCode {
    let gui_args = parse_gui_arguments(cmd_line);
    let settings = get_settings();
    let launch_app_by_id_visible = Rc::new(Cell::new(false));
    let app_id = Rc::new(Cell::new(Option::<u32>::None));
    let app_unlocked_achievements_count = Rc::new(Cell::new(0usize));

    // Create the UI components for the app view
    let (
        app_stack,
        app_shimmer_image,
        app_label,
        _app_achievements_button,
        _app_stats_button,
        app_achievement_count_value,
        app_stats_count_value,
        app_type_value,
        app_developer_value,
        app_metacritic_value,
        app_metacritic_box,
        _app_sidebar,
        app_achievements_model,
        app_achievement_string_filter,
        app_stat_model,
        app_stat_string_filter,
        app_pane,
        achievements_manual_adjustement,
        _achievements_manual_spinbox,
        achievements_manual_start,
        cancel_timed_unlock,
        app_achievements_stack,
    ) = create_app_view(
        app_id.clone(),
        app_unlocked_achievements_count.clone(),
        application,
    );

    // Loading box
    let list_spinner = Spinner::builder().margin_end(5).spinning(true).build();
    let list_spinner_label = Label::builder().label("Loading...").build();
    let list_spinner_box = Box::builder().halign(Align::Center).build();
    list_spinner_box.append(&list_spinner);
    list_spinner_box.append(&list_spinner_label);

    // Empty search result box
    let app_list_no_result_icon = Image::from_icon_name("edit-find-symbolic");
    app_list_no_result_icon.set_icon_size(IconSize::Large);
    let app_list_no_result_label = Label::builder().build();
    let app_list_no_result_box = Box::builder()
        .spacing(20)
        .valign(Align::Center)
        .halign(Align::Center)
        .orientation(Orientation::Vertical)
        .build();
    app_list_no_result_box.append(&app_list_no_result_icon);
    app_list_no_result_box.append(&app_list_no_result_label);

    // Header bar
    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    let search_entry = SearchEntry::builder()
        .placeholder_text("Name or AppId (Ctrl+K)")
        .build();
    let back_button = Button::builder()
        .icon_name("go-previous")
        .sensitive(false)
        .build();
    let (
        context_menu_button,
        _,
        menu_model,
        context_menu_button_loading,
        context_menu_button_loading_progress_label,
        context_menu_button_info_label,
    ) = create_context_menu_button();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&context_menu_button);
    header_bar.pack_end(&context_menu_button_loading);

    let list_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .build();

    let list_of_apps_or_no_result = Stack::builder()
        .transition_type(StackTransitionType::Crossfade)
        .build();
    list_of_apps_or_no_result.add_named(&list_scrolled_window, Some("list"));
    list_of_apps_or_no_result.add_named(&app_list_no_result_box, Some("empty"));

    // Main application stack component
    let list_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    list_stack.add_named(&list_spinner_box, Some("loading"));
    list_stack.add_named(&list_of_apps_or_no_result, Some("list"));
    list_stack.add_named(&app_pane, Some("app"));

    // App list models
    let list_factory = SignalListItemFactory::new();
    let list_store = ListStore::new::<GSteamAppObject>();
    let filter_junk_action = SimpleAction::new_stateful(
        "filter_junk_option",
        None,
        &settings.boolean("filter-junk").to_variant(),
    );
    let search_entry_clone = search_entry.clone();
    let filter_junk_action_clone = filter_junk_action.clone();
    let list_custom_filter = gtk::CustomFilter::new(move |obj| {
        let app = obj.downcast_ref::<GSteamAppObject>().unwrap();

        let hide_junk = filter_junk_action_clone
            .state()
            .and_then(|s| s.get::<bool>())
            .unwrap_or(false);

        if hide_junk && app.app_type() == "Junk" {
            return false;
        }

        let search_text = search_entry_clone.text().to_lowercase();
        if search_text.is_empty() {
            return true;
        }

        app.app_name().to_lowercase().contains(&search_text)
    });

    filter_junk_action.connect_activate(clone!(
        #[weak]
        list_custom_filter,
        #[strong]
        settings,
        move |action, _| {
            let state = action.state().unwrap();
            let value: bool = state.get::<bool>().unwrap();
            action.set_state(&(!value).to_variant());
            if let Err(e) = settings.set_boolean("filter-junk", !value) {
                eprintln!("[CLIENT] Error saving filter-junk setting: {e:?}");
            }
            list_custom_filter.changed(gtk::FilterChange::Different);
        }
    ));

    let list_filter_model = FilterListModel::builder()
        .model(&list_store)
        .filter(&list_custom_filter)
        .build();
    let list_selection_model = MultiSelection::new(Some(list_filter_model.clone()));
    list_selection_model.set_model(Some(&list_filter_model));
    let grid_view = GridView::builder()
        .min_columns(2)
        .margin_start(10)
        .margin_end(10)
        .css_name("unstyled-gridview")
        .model(&list_selection_model)
        .factory(&list_factory)
        .build();

    let window = ApplicationWindow::builder()
        .application(application)
        .title("SamRewritten")
        .default_width(904) // Somehow.. min width with default theme
        .default_height(600)
        .child(&list_stack)
        .titlebar(&header_bar)
        .build();

    let about_dialog = create_about_dialog(&window);

    // Connect list view activation
    grid_view.connect_activate(clone!(
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        menu_model,
        #[weak]
        app_achievement_count_value,
        #[weak]
        app_stats_count_value,
        #[weak]
        app_type_value,
        #[weak]
        app_developer_value,
        #[weak]
        app_metacritic_value,
        #[weak]
        app_metacritic_box,
        #[weak]
        app_stack,
        #[weak]
        list_stack,
        #[weak]
        app_label,
        #[weak]
        app_shimmer_image,
        move |list_view, position| {
            let Some(model) = list_view.model() else {
                return;
            };
            let Some(item) = model.item(position).and_downcast::<GSteamAppObject>() else {
                return;
            };

            switch_from_app_list_to_app(
                &item,
                application.clone(),
                &app_type_value,
                &app_developer_value,
                &app_achievement_count_value,
                &app_stats_count_value,
                app_stack.clone(),
                &app_id,
                &app_metacritic_box,
                &app_metacritic_value,
                &app_shimmer_image,
                &app_label,
                &menu_model,
                &list_stack,
            );
        }
    ));

    list_factory.connect_setup(move |_, list_item| {
        let entry = SteamAppCard::default();
        entry.set_size_request(400, 150);
        entry.set_margin_start(5);
        entry.set_margin_end(5);
        entry.set_margin_top(5);
        entry.set_margin_bottom(5);

        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        list_item.set_activatable(false);
        list_item.set_child(Some(&entry));
        list_item
            .property_expression("item")
            .bind(&entry, "app-object", Widget::NONE);
        list_item
            .property_expression("selected")
            .bind(&entry, "is-selected", Widget::NONE);
    });

    list_factory.connect_bind(clone!(
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        menu_model,
        #[weak]
        app_achievement_count_value,
        #[weak]
        app_stats_count_value,
        #[weak]
        app_type_value,
        #[weak]
        app_developer_value,
        #[weak]
        app_metacritic_value,
        #[weak]
        app_metacritic_box,
        #[weak]
        app_stack,
        #[weak]
        list_stack,
        #[weak]
        app_label,
        #[weak]
        app_shimmer_image,
        move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            let steam_app_object = list_item
                .item()
                .and_then(|item| item.downcast::<GSteamAppObject>().ok())
                .expect("Item should be a GSteamAppObject");
            let app_id_to_bind = steam_app_object.app_id();

            let card = list_item
                .child()
                .and_downcast::<SteamAppCard>()
                .expect("Child should be a SteamAppCard");
            let manage_button = card.manage_button();
            let manage_button_new_window = card.manage_button_new();
            let launch_button = card.launch_button();

            let handler = manage_button.connect_clicked(clone!(
                #[strong]
                app_id,
                #[weak]
                application,
                move |_| {
                    switch_from_app_list_to_app(
                        &steam_app_object,
                        application,
                        &app_type_value,
                        &app_developer_value,
                        &app_achievement_count_value,
                        &app_stats_count_value,
                        app_stack.clone(),
                        &app_id,
                        &app_metacritic_box,
                        &app_metacritic_value,
                        &app_shimmer_image,
                        &app_label,
                        &menu_model,
                        &list_stack,
                    );
                }
            ));

            unsafe {
                manage_button.set_data("handler", handler.as_raw());
            }

            let handler = manage_button_new_window.connect_clicked(move |_| {
                // TODO: Find a way to gracefully wait for this
                Command::new(get_executable_path())
                    .arg(format!("--auto-open={app_id_to_bind}"))
                    .spawn()
                    .expect("Could not start child process");
            });

            unsafe {
                manage_button_new_window.set_data("handler", handler.as_raw());
            }

            let handler = launch_button.connect_clicked(move |_| {
                #[cfg(unix)]
                {
                    Command::new("xdg-open")
                        .arg(format!("steam://run/{app_id_to_bind}"))
                        .spawn()
                        .expect("Could not start child process")
                        .wait()
                        .expect("Failed to wait on child process");
                }

                #[cfg(windows)]
                {
                    Command::new("cmd")
                        .arg("/C")
                        .arg("start")
                        .arg(&format!("steam://run/{app_id_to_bind}"))
                        .spawn()
                        .expect("Could not start child process")
                        .wait()
                        .expect("Failed to wait on child process");
                }
            });

            unsafe {
                launch_button.set_data("handler", handler.as_raw());
            }

            let handler_id = card.connect_is_selected_notify(clone!(
                #[weak]
                list_item,
                #[weak]
                list_selection_model,
                move |card| {
                    let position = list_item.position();
                    if position == u32::MAX {
                        return;
                    }

                    if card.is_selected() {
                        list_selection_model.select_item(position, false);
                        set_app_action_enabled(&application, "unlock_all_apps", true);
                        set_app_action_enabled(&application, "lock_all_apps", true);
                    } else {
                        list_selection_model.unselect_item(position);
                        let selection = list_selection_model.selection();
                        let has_selection = !selection.is_empty();
                        set_app_action_enabled(&application, "unlock_all_apps", has_selection);
                        set_app_action_enabled(&application, "lock_all_apps", has_selection);
                    }
                }
            ));

            unsafe {
                card.set_data("selection-handler", handler_id.as_raw());
            }
        }
    ));

    list_factory.connect_unbind(move |_, list_item| {
        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");

        let card = list_item
            .child()
            .and_then(|child| child.downcast::<SteamAppCard>().ok())
            .expect("Child should be a SteamAppCard");
        let manage_button = card.manage_button();
        let manage_button_new_window = card.manage_button_new();
        let launch_button = card.launch_button();

        unsafe {
            if let Some(handler) = manage_button.data("handler") {
                let ulong: c_ulong = *handler.as_ptr();
                let signal_handler = SignalHandlerId::from_glib(ulong);
                manage_button.disconnect(signal_handler);
            } else {
                eprintln!("[CLIENT] Manage button unbind failed");
            }

            if let Some(handler) = manage_button_new_window.data("handler") {
                let ulong: c_ulong = *handler.as_ptr();
                let signal_handler = SignalHandlerId::from_glib(ulong);
                manage_button_new_window.disconnect(signal_handler);
            } else {
                eprintln!("[CLIENT] Manage button new window unbind failed");
            }

            if let Some(handler) = launch_button.data("handler") {
                let ulong: c_ulong = *handler.as_ptr();
                let signal_handler = SignalHandlerId::from_glib(ulong);
                launch_button.disconnect(signal_handler);
            } else {
                eprintln!("[CLIENT] Launch button unbind failed");
            }

            if let Some(handler) = card.data("selection-handler") {
                let ulong: c_ulong = *handler.as_ptr();
                let signal_handler = SignalHandlerId::from_glib(ulong);
                card.disconnect(signal_handler);
            } else {
                eprintln!("[CLIENT] Card selection state unbind failed");
            }
        }
    });

    // Search entry setup
    search_entry.connect_search_changed(clone!(
        #[weak]
        list_custom_filter,
        #[weak]
        app_stat_string_filter,
        #[weak]
        app_achievement_string_filter,
        #[weak]
        list_store,
        move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());

            // This logic is needed to have flashes of "no results found"
            if launch_app_by_id_visible.take() {
                if let Some(app_id) = text.as_ref().and_then(|t| t.parse::<u32>().ok()) {
                    launch_app_by_id_visible.set(true);
                    list_store.insert(
                        1,
                        &GSteamAppObject::new(AppModel {
                            app_id,
                            app_name: format!("App {app_id}"),
                            app_type: AppModelType::App,
                            developer: "Unknown".to_string(),
                            image_url: None,
                            metacritic_score: None,
                        }),
                    );
                }

                app_achievement_string_filter.set_search(text.as_deref());
                app_stat_string_filter.set_search(text.as_deref());
                list_custom_filter.changed(gtk::FilterChange::Different);
                list_store.remove(0);
                return;
            }

            if let Some(app_id) = text.clone().and_then(|t| t.parse::<u32>().ok()) {
                launch_app_by_id_visible.set(true);
                list_store.insert(
                    0,
                    &GSteamAppObject::new(AppModel {
                        app_id,
                        app_name: format!("App {app_id}"),
                        app_type: AppModelType::App,
                        developer: "Unknown".to_string(),
                        image_url: None,
                        metacritic_score: None,
                    }),
                );
            }

            app_achievement_string_filter.set_search(text.as_deref());
            app_stat_string_filter.set_search(text.as_deref());
            list_custom_filter.changed(gtk::FilterChange::Different);
        }
    ));

    list_filter_model.connect_items_changed(clone!(
        #[weak]
        list_of_apps_or_no_result,
        move |model, _, _, _| {
            if model.n_items() == 0 {
                list_of_apps_or_no_result.set_visible_child_name("empty");
            } else {
                list_of_apps_or_no_result.set_visible_child_name("list");
            }
        }
    ));

    // Back button handler
    back_button.connect_clicked(clone!(
        #[weak]
        list_stack,
        #[weak]
        app_id,
        #[weak]
        menu_model,
        #[weak]
        application,
        #[weak]
        app_achievements_model,
        #[weak]
        app_stat_model,
        #[strong]
        cancel_timed_unlock,
        move |_| {
            cancel_timed_unlock.store(true, std::sync::atomic::Ordering::Relaxed);
            list_stack.set_visible_child_name("list");
            set_context_popover_to_app_list_context(&menu_model, &application);
            if let Some(app_id) = app_id.take() {
                spawn_blocking(move || {
                    let _ = StopApp { app_id }.request();
                });
            }

            // Clear achievements and stats for performance, but wait a bit before doing so
            // to avoid flashes of the data disappearing during the animation
            let handle = spawn_blocking(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
            });

            MainContext::default().spawn_local(async move {
                if Some(()) != handle.await.ok() {
                    eprintln!("[CLIENT] Threading task failed");
                }

                app_achievements_model.remove_all();
                app_stat_model.remove_all();
            });
        }
    ));

    // App actions
    #[cfg(feature = "adwaita")]
    {
        let default_theme = settings.string("app-theme").to_variant();
        let theme_action = SimpleAction::new_stateful(
            "change_theme",
            Some(&glib::VariantTy::STRING),
            &default_theme,
        );

        theme_action.connect_activate(move |action, parameter| {
            if let Some(theme_name) = parameter.and_then(|p| p.get::<String>()) {
                action.set_state(&theme_name.to_variant());
                let style_manager = adw::StyleManager::default();
                match theme_name.as_str() {
                    "dark" => style_manager.set_color_scheme(adw::ColorScheme::PreferDark),
                    "light" => style_manager.set_color_scheme(adw::ColorScheme::PreferLight),
                    _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
                }
                if let Err(e) = settings.set_string("app-theme", &theme_name) {
                    eprintln!("[CLIENT] Error saving app-theme setting: {e:?}");
                }
            }
        });

        theme_action.activate(Some(&default_theme));
        application.add_action(&theme_action);
    }

    #[cfg(not(feature = "adwaita"))]
    {
        let default_theme = match settings.string("app-theme").as_str() {
            "dark" => "dark",
            _ => "light",
        }
        .to_variant();

        let theme_action = SimpleAction::new_stateful(
            "change_theme",
            Some(glib::VariantTy::STRING),
            &default_theme,
        );

        theme_action.connect_activate(move |action, parameter| {
            if let Some(theme_name) = parameter.and_then(|p| p.get::<String>()) {
                action.set_state(&theme_name.to_variant());

                let default_settings =
                    gtk::Settings::default().expect("Could not get default settings");
                match theme_name.as_str() {
                    "dark" => {
                        default_settings.set_property("gtk-application-prefer-dark-theme", true);
                    }
                    _ => {
                        default_settings.set_property("gtk-application-prefer-dark-theme", false);
                    }
                }

                if let Err(e) = settings.set_string("app-theme", &theme_name) {
                    eprintln!("[CLIENT] Error saving app-theme setting: {e:?}");
                }
            }
        });

        theme_action.activate(Some(&default_theme));
        application.add_action(&theme_action);
    }

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
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            // TODO: rewrite with MainContext::channel when upgrading GTK version
            let (tx, rx) = std::sync::mpsc::channel::<(u32, String)>();
            let total_apps = apps_to_unlock.len();

            let handle = spawn_blocking(move || {
                let mut failed_apps = Vec::new();
                for (i, (app_id, app_name)) in apps_to_unlock.into_iter().enumerate() {
                    let current_step: u32 = (i as u32) + 1;
                    crate::dev_println!(
                        "[CLIENT] Unlocking app {app_id} ({current_step}/{total_apps})"
                    );

                    if let Err(e) = tx.send((current_step, app_name.to_string())) {
                        eprintln!("[CLIENT] Error sending app unlocked step: {e:?}");
                    };

                    let res = UnlockAllAchievements { app_id }.request();

                    if let Err(e) = res {
                        eprintln!("[CLIENT] Error unlocking app {}: {}", app_id, e);
                        failed_apps.push(app_name);
                    }
                }

                if let Err(e) = tx.send((u32::MAX, "DONE".to_string())) {
                    eprintln!("[CLIENT] Error sending done signal for unlocking: {e:?}");
                }

                failed_apps
            });

            glib::idle_add_local(move || {
                while let Ok((step, app_name)) = rx.try_recv() {
                    if step == u32::MAX {
                        return glib::ControlFlow::Break;
                    }

                    context_menu_button_loading_progress_label
                        .set_text(&format!("Unlocking {}/{}", step, total_apps));
                    context_menu_button_info_label.set_text(&app_name);
                }

                glib::ControlFlow::Continue
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

                        let dialog = gtk::MessageDialog::builder()
                            .message_type(gtk::MessageType::Error)
                            .buttons(gtk::ButtonsType::Ok)
                            .title("Unlock Incomplete")
                            .text(format!(
                                "Failed to unlock achievements for the following apps:\n\n{}",
                                display_text
                            ))
                            .build();

                        if let Some(current_window) = application.active_window() {
                            dialog.set_transient_for(Some(&current_window));
                        }

                        dialog.run_future().await;
                        dialog.close();
                    }

                    set_app_action_enabled(&application, "unlock_all_apps", true);
                    set_app_action_enabled(&application, "lock_all_apps", true);
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
            context_menu_button_loading.set_visible(true);
            context_menu_button.set_visible(false);
            grid_view.set_sensitive(false);

            // TODO: rewrite with MainContext::channel when upgrading GTK version
            let (tx, rx) = std::sync::mpsc::channel::<(u32, String)>();
            let total_apps = apps_to_lock.len();

            let handle = spawn_blocking(move || {
                for (i, (app_id, app_name)) in apps_to_lock.into_iter().enumerate() {
                    let current_step: u32 = (i as u32) + 1;
                    crate::dev_println!(
                        "[CLIENT] Locking app {app_id} ({current_step}/{total_apps})"
                    );

                    if let Err(e) = tx.send((current_step, app_name.to_string())) {
                        eprintln!("[CLIENT] Error sending app locked step: {e:?}");
                    };

                    let res = ResetStats {
                        app_id,
                        achievements_too: true,
                    }
                    .request();

                    if let Err(e) = res {
                        eprintln!("[CLIENT] Error locking app {}: {}", app_id, e);
                        if let Err(e) = tx.send((u32::MAX, "ERROR".to_string())) {
                            eprintln!("[CLIENT] Error sending stop signal for unlocking: {e:?}");
                        }
                        return Err(e);
                    }
                }

                if let Err(e) = tx.send((u32::MAX, "DONE".to_string())) {
                    eprintln!("[CLIENT] Error sending done signal for locking: {e:?}");
                }
                Ok(())
            });

            glib::idle_add_local(move || {
                while let Ok((step, app_name)) = rx.try_recv() {
                    if step == u32::MAX {
                        return glib::ControlFlow::Break;
                    }

                    context_menu_button_loading_progress_label
                        .set_text(&format!("Locking {}/{}", step, total_apps));
                    context_menu_button_info_label.set_text(&app_name);
                }

                glib::ControlFlow::Continue
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
                    context_menu_button_loading.set_visible(false);
                    context_menu_button.set_visible(true);
                    grid_view.set_sensitive(true);
                }
            ));
        }
    ));

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
            let apps = spawn_blocking(move || GetSubscribedAppList.request());
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

            MainContext::default().spawn_local(clone!(async move {
                let Ok(Ok(_success)) = handle.await else {
                    return app_stack.set_visible_child_name("failed");
                };

                action_refresh_achievements_list.activate(None);
            }));
        }
    ));

    list_stack.connect_visible_child_notify(clone!(
        #[weak]
        back_button,
        #[weak]
        application,
        #[weak]
        app_stack,
        #[weak]
        search_entry,
        #[weak]
        action_refresh_app_list,
        move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                back_button.set_sensitive(false);
                action_refresh_app_list.set_enabled(false);
            } else if stack.visible_child_name().as_deref() == Some("app") {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("Achievement or stat..."));
                back_button.set_sensitive(true);
                action_refresh_app_list.set_enabled(false);
            } else {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("Name or AppId (Ctrl+K)"));
                back_button.set_sensitive(false);
                action_refresh_app_list.set_enabled(true);

                let auto_launch_app = gui_args.auto_open.get();
                if auto_launch_app > 0 {
                    gui_args.auto_open.set(0);

                    // let mut found_iter = None;
                    for ach in &list_store {
                        if let Ok(obj) = ach {
                            let g_app = obj
                                .downcast::<GSteamAppObject>()
                                .expect("Not a GSteamAppObject");
                            if g_app.app_id() == auto_launch_app {
                                // found_iter = Some(g_app);
                                switch_from_app_list_to_app(
                                    &g_app,
                                    application.clone(),
                                    &app_type_value,
                                    &app_developer_value,
                                    &app_achievement_count_value,
                                    &app_stats_count_value,
                                    app_stack.clone(),
                                    &app_id,
                                    &app_metacritic_box,
                                    &app_metacritic_value,
                                    &app_shimmer_image,
                                    &app_label,
                                    &menu_model,
                                    stack,
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }
    ));

    app_stack.set_visible_child_name("loading");
    list_stack.set_visible_child_name("loading");
    action_refresh_app_list.activate(None);
    action_refresh_app_list.set_enabled(false);

    setup_app_actions(
        application,
        &about_dialog,
        &action_refresh_app_list,
        &action_refresh_achievements_list,
        &action_clear_all_stats_and_achievements,
        &filter_junk_action,
        &action_select_all_apps,
        &action_unselect_all_apps,
        &action_unlock_all_selected,
        &action_lock_all_selected,
    );

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(clone!(
        #[weak]
        search_entry,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_controller, key, _keycode, state| {
            if state.contains(gtk::gdk::ModifierType::CONTROL_MASK) && key == gtk::gdk::Key::k {
                search_entry.grab_focus();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    ));

    window.add_controller(key_controller);

    warn(&window);

    window.present();

    ExitCode::SUCCESS
}

#[cfg(unix)]
fn warn(window: &ApplicationWindow) {
    use crate::utils::steam_locator::SteamLocator;
    use gtk::prelude::{DialogExt, GtkWindowExt, WidgetExt};

    let dirs = SteamLocator::get_local_steam_install_root_folders();
    if dirs.len() > 1 {
        let path_list = dirs
            .iter()
            .map(|p| format!("• {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        let full_message = format!(
            "Multiple Steam installations have been detected on your system. \
            This will most likely cause <b>severe instabilities</b> with SamRewritten. \
            <b>Please delete all unused Steam installations before proceeding or use \
            environment variables to point SamRewritten to the correct location, \
            as described on the <a href=\"https://github.com/PaulCombal/SamRewritten?tab=readme-ov-file#environment-variables\">Github main page.</a></b>\n\n\
            The following locations were found:\n{}",
            path_list
        );

        let dialog = gtk::MessageDialog::new(
            Some(window),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Warning,
            gtk::ButtonsType::Ok,
            full_message,
        );

        dialog.set_use_markup(true);
        dialog.set_title(Some("WARNING"));
        dialog.connect_response(|dialog, _| {
            dialog.destroy();
        });

        dialog.show();
    }
}

#[cfg(windows)]
fn warn(_window: &ApplicationWindow) {}
