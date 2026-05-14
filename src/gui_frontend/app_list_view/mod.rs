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

mod bulk_actions;
mod refresh_actions;
mod settings_bindings;

use crate::backend::app_lister::{AppModel, AppModelType};
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::app_list_view_callbacks::switch_from_app_list_to_app;
use crate::gui_frontend::app_view::create_app_view;
use crate::gui_frontend::application_actions::{set_app_action_enabled, setup_app_actions};
use crate::gui_frontend::dialogs::warn;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::request::{LaunchApp, Request, StopApp};
use crate::gui_frontend::ui_components::{
    create_about_dialog, create_context_menu_button, set_context_popover_to_app_list_context,
};
use crate::gui_frontend::widgets::steam_app_card::SteamAppCard;
use crate::utils::app_paths::get_executable_path;
use crate::utils::arguments::parse_gui_arguments;
use bulk_actions::create_bulk_actions;
use gtk::gio::{ApplicationCommandLine, ListStore, spawn_blocking};
use gtk::glib::ExitCode;
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box, Button, CustomSorter, FilterListModel, GridView, HeaderBar,
    Image, Label, ListItem, MultiSelection, Orientation, PolicyType, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SortListModel, Spinner, Stack, StackTransitionType, Widget,
};
use gtk::{IconSize, glib};
use refresh_actions::{
    create_clear_all_action, create_refresh_achievements_action, create_refresh_app_list_action,
};
use settings_bindings::setup_settings_bindings;
use std::cell::{Cell, RefCell};
use std::process::Command;
use std::rc::Rc;

const MAX_CONCURRENT_IDLE: usize = 30;

/// Recount how many apps are idling and propagate the resulting "can start
/// idling?" decision onto every app in the store. Cards bind their idle
/// button's `sensitive` property to this; when the cap is reached, every
/// non-idling app's idle button greys out.
fn recompute_idle_cap(list_store: &ListStore) {
    let mut count = 0usize;
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>()
            && app.is_idling()
        {
            count += 1;
        }
    }
    let can_start = count < MAX_CONCURRENT_IDLE;
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>()
            && app.can_start_idling() != can_start
        {
            app.set_can_start_idling(can_start);
        }
    }
}

pub fn create_main_ui(
    application: &MainApplication,
    cmd_line: &ApplicationCommandLine,
) -> ExitCode {
    #[cfg(unix)]
    if let Ok(appdir) = std::env::var("APPDIR") {
        if let Some(display) = gtk::gdk::Display::default() {
            let theme = gtk::IconTheme::for_display(&display);

            if !theme.has_icon("open-menu-symbolic") {
                crate::dev_println!("[CLIENT] Icon not found in system theme. Using fallback.");

                let fallback_path = std::path::Path::new(&appdir).join("icons");
                theme.add_search_path(fallback_path);
            }
        }
    }

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

    // Hot-path caches — avoid repeated GSettings reads and to_lowercase() allocations
    // inside the filter/sort closures. Updated by their respective change handlers.
    let filter_junk_cache: Rc<Cell<bool>> = Rc::new(Cell::new(settings.boolean("filter-junk")));
    let sort_mode_cache: Rc<RefCell<String>> =
        Rc::new(RefCell::new(settings.string("app-sort").to_string()));
    let search_text_lower: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let list_custom_filter = gtk::CustomFilter::new(clone!(
        #[strong]
        filter_junk_cache,
        #[strong]
        search_text_lower,
        move |obj| {
            let app = obj.downcast_ref::<GSteamAppObject>().unwrap();

            if filter_junk_cache.get() && app.is_junk() {
                return false;
            }

            let search_text = search_text_lower.borrow();
            if search_text.is_empty() {
                return true;
            }

            app.lowercase_name().contains(search_text.as_str())
        }
    ));

    let list_filter_model = FilterListModel::builder()
        .model(&list_store)
        .filter(&list_custom_filter)
        .build();

    let list_custom_sorter = CustomSorter::new(clone!(
        #[strong]
        sort_mode_cache,
        move |a, b| {
            let a = a.downcast_ref::<GSteamAppObject>().unwrap();
            let b = b.downcast_ref::<GSteamAppObject>().unwrap();
            let alphabetical = || {
                let a_name = a.lowercase_name();
                let b_name = b.lowercase_name();
                a_name.as_str().cmp(b_name.as_str())
            };
            let ord = match sort_mode_cache.borrow().as_str() {
                "alphabetical" => alphabetical(),
                "last_played" => b
                    .last_played()
                    .cmp(&a.last_played())
                    .then_with(alphabetical),
                "playtime" => b
                    .playtime_minutes()
                    .cmp(&a.playtime_minutes())
                    .then_with(alphabetical),
                _ => a.app_id().cmp(&b.app_id()),
            };
            ord.into()
        }
    ));

    setup_settings_bindings(
        application,
        &settings,
        &list_custom_filter,
        &list_custom_sorter,
        filter_junk_cache.clone(),
        sort_mode_cache.clone(),
    );

    let list_sort_model = SortListModel::builder()
        .model(&list_filter_model)
        .sorter(&list_custom_sorter)
        .build();

    let list_selection_model = MultiSelection::new(Some(list_sort_model.clone()));
    list_selection_model.set_model(Some(&list_sort_model));
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

    // Install everything once per allocated card. Handlers read the *current*
    // bound app via `card.app_object()` (kept in sync by the property-expression
    // binding below), and the selection handler captures the ListItem weakly.
    // This means no per-bind closure allocation or signal (re)installation
    // during scroll — the work happens ~once per visible-slot widget instance.
    list_factory.connect_setup(clone!(
        #[strong]
        app_id,
        #[weak]
        application,
        #[weak]
        list_store,
        #[weak]
        list_selection_model,
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
            let card = SteamAppCard::default();
            card.set_size_request(400, 150);
            card.set_margin_start(5);
            card.set_margin_end(5);
            card.set_margin_top(5);
            card.set_margin_bottom(5);

            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            list_item.set_activatable(false);
            list_item.set_child(Some(&card));
            list_item
                .property_expression("item")
                .bind(&card, "app-object", Widget::NONE);
            list_item
                .property_expression("selected")
                .bind(&card, "is-selected", Widget::NONE);

            card.manage_button().connect_clicked(clone!(
                #[weak]
                card,
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
                move |_| {
                    let Some(steam_app_object) = card.app_object() else {
                        return;
                    };
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

            card.manage_button_new().connect_clicked(clone!(
                #[weak]
                card,
                move |_| {
                    let Some(app) = card.app_object() else {
                        return;
                    };
                    let app_id_to_bind = app.app_id();
                    // TODO: Find a way to gracefully wait for this
                    Command::new(get_executable_path())
                        .arg(format!("--auto-open={app_id_to_bind}"))
                        .spawn()
                        .expect("Could not start child process");
                }
            ));

            card.idle_button().connect_toggled(clone!(
                #[weak]
                card,
                #[weak]
                list_store,
                move |button| {
                    let Some(app) = card.app_object() else {
                        return;
                    };
                    let active = button.is_active();
                    // If the toggle already agrees with the app state, the change came from
                    // the property-expression sync on cell rebind — not a user action.
                    if active == app.is_idling() {
                        return;
                    }

                    let app_id = app.app_id();
                    app.set_is_idling(active);
                    recompute_idle_cap(&list_store);

                    let handle = spawn_blocking(move || {
                        if active {
                            LaunchApp { app_id }.request().map(|_| ())
                        } else {
                            StopApp { app_id }.request().map(|_| ())
                        }
                    });

                    MainContext::default().spawn_local(clone!(
                        #[weak]
                        list_store,
                        async move {
                            if let Ok(Err(e)) = handle.await {
                                eprintln!(
                                    "[CLIENT] {} app {app_id} failed: {e:?}",
                                    if active { "Launching" } else { "Stopping" }
                                );
                                app.set_is_idling(!active);
                                recompute_idle_cap(&list_store);
                            }
                        }
                    ));
                }
            ));

            card.launch_button().connect_clicked(clone!(
                #[weak]
                card,
                move |_| {
                    let Some(app) = card.app_object() else {
                        return;
                    };
                    let app_id_to_bind = app.app_id();
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
                }
            ));

            card.connect_is_selected_notify(clone!(
                #[weak]
                list_item,
                #[weak]
                list_selection_model,
                #[weak]
                application,
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
        }
    ));

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
        #[strong]
        search_text_lower,
        move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());
            // Refresh the lowercased cache that the list filter reads on every item.
            *search_text_lower.borrow_mut() =
                text.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();

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
                            playtime_minutes: None,
                            last_played: None,
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
                        playtime_minutes: None,
                        last_played: None,
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

    let (
        action_select_all_apps,
        action_unselect_all_apps,
        action_unlock_all_selected,
        action_lock_all_selected,
    ) = create_bulk_actions(
        application,
        &grid_view,
        &context_menu_button,
        &context_menu_button_loading,
        &context_menu_button_loading_progress_label,
        &context_menu_button_info_label,
    );

    let action_refresh_app_list = create_refresh_app_list_action(
        application,
        &grid_view,
        &list_store,
        &list_scrolled_window,
        &list_of_apps_or_no_result,
        &app_list_no_result_label,
        &list_stack,
        &search_entry,
    );

    let action_refresh_achievements_list = create_refresh_achievements_action(
        application,
        &app_id,
        &app_unlocked_achievements_count,
        &app_achievements_model,
        &app_stat_model,
        &app_achievement_count_value,
        &app_stats_count_value,
        &app_stack,
        &achievements_manual_adjustement,
        &achievements_manual_start,
        &app_achievements_stack,
        &cancel_timed_unlock,
    );

    let action_clear_all_stats_and_achievements = create_clear_all_action(
        application,
        &app_id,
        &app_achievements_model,
        &app_stat_model,
        &action_refresh_achievements_list,
        &app_stack,
    );

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

                    let target_app = list_store
                        .snapshot()
                        .into_iter()
                        .filter_map(|obj| obj.downcast::<GSteamAppObject>().ok())
                        .find(|g_app| g_app.app_id() == auto_launch_app);

                    let app_to_open = target_app.unwrap_or_else(|| {
                        GSteamAppObject::new(AppModel {
                            app_id: auto_launch_app,
                            app_name: format!("App {auto_launch_app}"),
                            app_type: AppModelType::App,
                            developer: "Unknown".to_string(),
                            image_url: None,
                            metacritic_score: None,
                            playtime_minutes: None,
                            last_played: None,
                        })
                    });

                    switch_from_app_list_to_app(
                        &app_to_open,
                        application,
                        &app_type_value,
                        &app_developer_value,
                        &app_achievement_count_value,
                        &app_stats_count_value,
                        app_stack,
                        &app_id,
                        &app_metacritic_box,
                        &app_metacritic_value,
                        &app_shimmer_image,
                        &app_label,
                        &menu_model,
                        stack,
                    );
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
