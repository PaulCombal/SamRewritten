// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
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

use super::stat::GStatObject;
use crate::backend::app_lister::{AppModel, AppModelType};
use crate::frontend::MainApplication;
use crate::frontend::achievement::GAchievementObject;
use crate::frontend::app_view::create_app_view;
use crate::frontend::application_actions::{set_app_action_enabled, setup_app_actions};
use crate::frontend::request::{
    GetAchievements, GetOwnedAppList, GetStats, LaunchApp, Request, ResetStats, StopApp,
};
use crate::frontend::shimmer_image::ShimmerImage;
use crate::frontend::steam_app::GSteamAppObject;
use crate::frontend::ui_components::{
    create_about_dialog, create_context_menu_button, set_context_popover_to_app_details_context,
    set_context_popover_to_app_list_context,
};
use crate::utils::ipc_types::SamError;
use gtk::gio::{ListStore, SimpleAction, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box, Button, FilterListModel, HeaderBar, Image, Label, ListItem,
    ListView, NoSelection, Orientation, PolicyType, ScrolledWindow, SearchEntry,
    SignalListItemFactory, Spinner, Stack, StackTransitionType, StringFilter,
    StringFilterMatchMode, Widget,
};
use gtk::{IconSize, glib};
use std::cell::Cell;
use std::rc::Rc;

pub fn create_main_ui(application: &MainApplication) {
    let launch_app_by_id_visible = Rc::new(Cell::new(false));
    let app_id = Rc::new(Cell::new(Option::<u32>::None));

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
    ) = create_app_view(app_id.clone());

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
        .placeholder_text("App name or App Id")
        .build();
    let back_button = Button::builder()
        .icon_name("go-previous")
        .sensitive(false)
        .build();
    let (context_menu_button, _, menu_model) = create_context_menu_button();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&context_menu_button);

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
    let list_string_filter = StringFilter::builder()
        .expression(&GSteamAppObject::this_expression("app_name"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let list_filter_model = FilterListModel::builder()
        .model(&list_store)
        .filter(&list_string_filter)
        .build();
    let list_selection_model = NoSelection::new(Option::<ListStore>::None);
    list_selection_model.set_model(Some(&list_filter_model));
    let list_view = ListView::builder()
        .single_click_activate(true)
        .orientation(Orientation::Vertical)
        .show_separators(true)
        .model(&list_selection_model)
        .factory(&list_factory)
        .build();

    let window = ApplicationWindow::builder()
        .application(application)
        .title("SamRewritten")
        .default_width(800)
        .default_height(600)
        .child(&list_stack)
        .titlebar(&header_bar)
        .build();

    let about_dialog = create_about_dialog(&window);

    // Connect list view activation
    list_view.connect_activate(clone!(
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        menu_model,
        #[weak]
        app_achievements_model,
        #[weak]
        app_stat_model,
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
        move |list_view, position| {
            let Some(model) = list_view.model() else {
                return;
            };
            let Some(item) = model.item(position).and_downcast::<GSteamAppObject>() else {
                return;
            };
            set_app_action_enabled(&application, "refresh_achievements_list", false);
            app_type_value.set_label("...");
            app_developer_value.set_label("...");
            app_achievement_count_value.set_label("...");
            app_stats_count_value.set_label("...");
            app_stack.set_visible_child_name("loading");
            app_achievements_model.remove_all();
            app_stat_model.remove_all();
            app_id.set(Some(item.app_id()));
            app_metacritic_box.set_visible(false);

            let app_type_copy = item.app_type();
            let app_id_copy = item.app_id();
            let app_developer_copy = item.developer();
            let app_metacritic_copy = item.metacritic_score();
            let handle = spawn_blocking(move || {
                let req = LaunchApp {
                    app_id: app_id_copy,
                }
                .request();

                match req {
                    Err(e) => {
                        eprintln!("[LAUNCH APP] Failed to launch app: {}", e);
                        return (Err(e), Err(SamError::UnknownError));
                    }
                    Ok(_) => {}
                }

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

            set_context_popover_to_app_details_context(&menu_model, &application);

            MainContext::default().spawn_local(clone!(async move {
                let Ok((Ok(achievements), Ok(stats))) = handle.await else {
                    return app_stack.set_visible_child_name("failed");
                };

                let achievement_len = achievements.len();
                app_stats_count_value.set_label(&format!("{}", stats.len()));
                app_achievement_count_value.set_label(&format!("{}", achievements.len()));
                achievements
                    .into_iter()
                    .map(GAchievementObject::new)
                    .for_each(|achievement| app_achievements_model.append(&achievement));
                stats
                    .into_iter()
                    .map(GStatObject::new)
                    .for_each(|stat| app_stat_model.append(&stat));
                app_type_value.set_label(&format!("{app_type_copy}"));
                app_developer_value.set_label(&app_developer_copy);
                app_metacritic_value.set_label(&format!("{app_metacritic_copy}"));

                if achievement_len > 0 {
                    app_stack.set_visible_child_name("achievements");
                } else {
                    app_stack.set_visible_child_name("empty");
                }

                if app_metacritic_copy != u8::MAX {
                    app_metacritic_box.set_visible(true);
                }

                set_app_action_enabled(&application, "refresh_achievements_list", true);
                set_app_action_enabled(&application, "clear_all_stats_and_achievements", true);
            }));

            if let Some(url) = item.image_url() {
                app_shimmer_image.set_url(url.as_str());
            } else {
                app_shimmer_image.reset();
            }
            app_label.set_markup(&format!(
                "<span font_desc=\"Bold 16\">{}</span>",
                item.app_name()
            ));
            list_stack.set_visible_child_name("app");
        }
    ));

    // List factory setup
    list_factory.connect_setup(move |_, list_item| {
        let image = ShimmerImage::new();
        let label = Label::builder().margin_start(20).build();
        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let icon = Image::builder().icon_name("pan-end").margin_end(20).build();
        let entry = Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(8)
            .margin_end(8)
            .build();
        entry.append(&image);
        entry.append(&label);
        entry.append(&spacer);
        entry.append(&icon);

        let list_item = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be a ListItem");
        list_item.set_child(Some(&entry));
        list_item
            .property_expression("item")
            .chain_property::<GSteamAppObject>("app_name")
            .bind(&label, "label", Widget::NONE);
        list_item
            .property_expression("item")
            .chain_property::<GSteamAppObject>("image_url")
            .bind(&image, "url", Widget::NONE);
    });

    // Search entry setup
    search_entry.connect_search_changed(clone!(
        #[weak]
        list_string_filter,
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
                if let Some(app_id) = text.as_ref().map(|t| t.parse::<u32>().ok()).flatten() {
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
                list_string_filter.set_search(text.as_deref());
                list_store.remove(0);
                return;
            }

            if let Some(app_id) = text.clone().map(|t| t.parse::<u32>().ok()).flatten() {
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
            list_string_filter.set_search(text.as_deref());
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
        move |_| {
            list_stack.set_visible_child_name("list");
            set_context_popover_to_app_list_context(&menu_model, &application);
            if let Some(app_id) = app_id.take() {
                spawn_blocking(move || {
                    let _ = StopApp { app_id }.request();
                });
            }
        }
    ));

    // App actions
    let action_refresh_app_list = SimpleAction::new("refresh_app_list", None);
    action_refresh_app_list.connect_activate(clone!(
        #[strong]
        list_view,
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
        move |_, _| {
            list_stack.set_visible_child_name("loading");
            search_entry.set_sensitive(false);
            let apps = spawn_blocking(move || GetOwnedAppList.request());
            MainContext::default().spawn_local(clone!(
                #[weak]
                list_view,
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
                                list_scrolled_window.set_child(Some(&list_view));
                                list_stack.set_visible_child_name("list");
                            } else {
                                list_store.remove_all();
                                let models: Vec<GSteamAppObject> =
                                    app_vec.into_iter().map(GSteamAppObject::new).collect();
                                list_store.extend_from_slice(&models);
                                list_scrolled_window.set_child(Some(&list_view));
                                list_stack.set_visible_child_name("list");
                                app_list_no_result_label.set_text("No results. Check for spelling mistakes or try typing an App Id.");
                            }
                        },
                        Ok(Err(sam_error)) if sam_error == SamError::AppListRetrievalFailed => {
                            search_entry.set_sensitive(true);
                            app_list_no_result_label.set_text("Failed to load library. Check your internet connection. Search for App Id to get started.");
                            list_of_apps_or_no_result.set_visible_child_name("empty");
                            list_scrolled_window.set_child(Some(&list_view));
                            list_stack.set_visible_child_name("list");
                        },
                        Ok(Err(sam_error)) => {
                            println!("Unknown error: {}", sam_error);
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
        move |_, _| {
            app_stack.set_visible_child_name("loading");
            set_app_action_enabled(&application, "refresh_achievements_list", false);
            app_achievements_model.remove_all();
            app_stat_model.remove_all();

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

            MainContext::default().spawn_local(clone!(async move {
                let Ok((Ok(achievements), Ok(stats))) = handle.await else {
                    return app_stack.set_visible_child_name("failed");
                };

                let achievement_len = achievements.len();
                app_stats_count_value.set_label(&format!("{}", stats.len()));
                app_achievement_count_value.set_label(&format!("{}", achievements.len()));
                achievements
                    .into_iter()
                    .map(GAchievementObject::new)
                    .for_each(|achievement| app_achievements_model.append(&achievement));
                stats
                    .into_iter()
                    .map(GStatObject::new)
                    .for_each(|stat| app_stat_model.append(&stat));

                if achievement_len > 0 {
                    app_stack.set_visible_child_name("achievements");
                } else {
                    app_stack.set_visible_child_name("empty");
                }

                set_app_action_enabled(&application, "refresh_achievements_list", true);
                set_app_action_enabled(&application, "clear_all_stats_and_achievements", true);
            }));
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
                let success = ResetStats {
                    app_id: app_id_copy,
                    achievements_too: true,
                }
                .request();
                success
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
        search_entry,
        #[weak]
        action_refresh_app_list,
        move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                back_button.set_sensitive(false);
                action_refresh_app_list.set_enabled(false);
            } else if stack.visible_child_name().as_deref() == Some("app") {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("App attribute..."));
                back_button.set_sensitive(true);
                action_refresh_app_list.set_enabled(false);
            } else {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some("App name..."));
                back_button.set_sensitive(false);
                action_refresh_app_list.set_enabled(true);
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
    );

    window.present();
}
