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

mod achievement_loader;
mod app_index;
mod bulk_actions;
mod progress_actions;
mod refresh_actions;
mod settings_bindings;
mod sidebar;

use crate::backend::app_lister::{AppModel, AppModelType};
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::app_list_view_callbacks::switch_from_app_list_to_app;
use crate::gui_frontend::app_view::create_app_view;
use crate::gui_frontend::application_actions::{
    set_app_action_enabled, set_timed_unlock_actions_enabled, setup_app_actions,
};
use crate::gui_frontend::dialogs::choose_steam_install_then;
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::profile_view::build_profile_view;
use crate::gui_frontend::profile_view::identity::{Identity, SharedIdentity, load_identity};
use crate::gui_frontend::request::{AppProgress, LaunchApp, Request, StopApp};
use crate::gui_frontend::ui_components::{
    create_context_menu_button, set_context_popover_to_app_list_context,
    set_context_popover_to_profile_context,
};
use crate::gui_frontend::widgets::steam_app_card::{CARD_HEIGHT, CARD_MIN_WIDTH, SteamAppCard};
use crate::utils::action_journal;
use crate::utils::app_paths::get_executable_path;
use crate::utils::arguments::parse_gui_arguments;
use achievement_loader::AchievementLoader;
use bulk_actions::create_bulk_actions;
use gtk::gio::{ApplicationCommandLine, ListStore, spawn_blocking};
use gtk::glib::ExitCode;
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box, Button, CustomSorter, FilterListModel, GridView, HeaderBar,
    Image, Label, ListItem, MultiSelection, Orientation, PolicyType, ScrolledWindow, SearchEntry,
    SignalListItemFactory, SortListModel, Spinner, Stack, StackTransitionType, ToggleButton,
    Widget,
};
use gtk::{IconSize, glib};
use progress_actions::create_progress_actions;
use refresh_actions::{
    create_clear_all_action, create_refresh_achievements_action, create_refresh_app_list_action,
};
use settings_bindings::setup_settings_bindings;
use sidebar::{build_sidebar, sort_needs_counts};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::process::Command;
use std::rc::Rc;

#[derive(Default)]
pub(super) struct FilterState {
    pub junk: Cell<bool>,
    pub only_idling: Cell<bool>,
    pub hide_fully_unlocked: Cell<bool>,
    pub hide_never_launched: Cell<bool>,
    pub hide_no_unlocked: Cell<bool>,
    pub hide_without_achievements: Cell<bool>,
}

impl FilterState {
    fn from_settings(settings: &gtk::gio::Settings) -> Self {
        let state = Self::default();
        state.reload(settings);
        state
    }

    pub(super) fn reload(&self, settings: &gtk::gio::Settings) {
        self.junk.set(settings.boolean("filter-junk"));
        self.only_idling.set(settings.boolean("filter-only-idling"));
        self.hide_fully_unlocked
            .set(settings.boolean("filter-hide-fully-unlocked"));
        self.hide_never_launched
            .set(settings.boolean("filter-hide-never-launched"));
        self.hide_no_unlocked
            .set(settings.boolean("filter-hide-no-unlocked"));
        self.hide_without_achievements
            .set(settings.boolean("filter-hide-without-achievements"));
    }

    fn depends_on_counts(&self) -> bool {
        self.hide_fully_unlocked.get()
            || self.hide_no_unlocked.get()
            || self.hide_without_achievements.get()
    }
}

const COUNT_FILTER_KEYS: &[&str] = &[
    "filter-hide-fully-unlocked",
    "filter-hide-no-unlocked",
    "filter-hide-without-achievements",
];

#[cfg(feature = "adwaita")]
const SIDEBAR_COLLAPSE_WIDTH: i32 = 1150;

#[cfg(feature = "adwaita")]
fn build_list_page(
    settings: &gtk::gio::Settings,
    sidebar: &Box,
    content: &Stack,
    toggle: &ToggleButton,
) -> gtk::Widget {
    use adw::prelude::*;

    let split = adw::OverlaySplitView::builder()
        .sidebar(sidebar)
        .content(content)
        .min_sidebar_width(f64::from(sidebar::SIDEBAR_WIDTH))
        .max_sidebar_width(f64::from(sidebar::SIDEBAR_WIDTH))
        .build();

    settings
        .bind("sidebar-visible", &split, "show-sidebar")
        .flags(gtk::gio::SettingsBindFlags::GET)
        .build();
    split.connect_show_sidebar_notify(clone!(
        #[strong]
        settings,
        move |split| {
            if !split.is_collapsed() {
                let _ = settings.set_boolean("sidebar-visible", split.shows_sidebar());
            }
        }
    ));
    split.connect_collapsed_notify(clone!(
        #[strong]
        settings,
        move |split| {
            if !split.is_collapsed() {
                split.set_show_sidebar(settings.boolean("sidebar-visible"));
            }
        }
    ));
    split
        .bind_property("show-sidebar", toggle, "active")
        .bidirectional()
        .sync_create()
        .build();

    let bin = adw::BreakpointBin::new();
    bin.set_child(Some(&split));
    bin.set_size_request(360, 200);

    let condition =
        adw::BreakpointCondition::parse(&format!("max-width: {SIDEBAR_COLLAPSE_WIDTH}px"))
            .expect("static breakpoint condition");
    let breakpoint = adw::Breakpoint::new(condition);
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    bin.add_breakpoint(breakpoint);

    bin.upcast()
}

#[cfg(not(feature = "adwaita"))]
fn build_list_page(
    settings: &gtk::gio::Settings,
    sidebar: &Box,
    content: &Stack,
    toggle: &ToggleButton,
) -> gtk::Widget {
    settings.bind("sidebar-visible", sidebar, "visible").build();
    settings.bind("sidebar-visible", toggle, "active").build();

    let page = Box::builder().orientation(Orientation::Horizontal).build();
    page.append(sidebar);
    page.append(content);
    page.upcast()
}

/// Maximum number of apps the GUI lets the user idle simultaneously. Unrelated
/// to bulk fan-out concurrency: idle sessions are long-running app-server
/// children that hold a Steam connection open, so this is a UX cap on
/// "fake-running" games, not an IPC throughput tuning.
const MAX_CONCURRENT_IDLE: usize = 8;

/// Full recount: scan the store, refresh `idle_count`, and propagate the
/// resulting "can start idling?" decision onto every app. Use this after the
/// store is repopulated (e.g. after a library refresh).
pub(super) fn recompute_idle_cap(list_store: &ListStore, idle_count: &Cell<usize>) {
    let mut count = 0usize;
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>()
            && app.is_idling()
        {
            count += 1;
        }
    }
    idle_count.set(count);
    let can_start = count < MAX_CONCURRENT_IDLE;
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>()
            && app.can_start_idling() != can_start
        {
            app.set_can_start_idling(can_start);
        }
    }
}

/// Apply a known delta (+1 / -1) to `idle_count`. Cards' `can_start_idling`
/// only flips when the count crosses `MAX_CONCURRENT_IDLE` — otherwise this is
/// O(1) and avoids the per-toggle full-store walk.
fn apply_idle_cap_delta(list_store: &ListStore, idle_count: &Cell<usize>, delta: i32) {
    let old_count = idle_count.get();
    let new_count = (old_count as i32 + delta).max(0) as usize;
    idle_count.set(new_count);
    let was_under = old_count < MAX_CONCURRENT_IDLE;
    let now_under = new_count < MAX_CONCURRENT_IDLE;
    if was_under == now_under {
        return;
    }
    let can_start = now_under;
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>()
            && app.can_start_idling() != can_start
        {
            app.set_can_start_idling(can_start);
        }
    }
}

pub type PrefetchedProgress = Rc<RefCell<Option<(u32, String, AppProgress)>>>;

pub fn create_main_ui(
    application: &MainApplication,
    cmd_line: &ApplicationCommandLine,
) -> ExitCode {
    crate::gui_frontend::i18n::init();

    #[cfg(unix)]
    if let Ok(appdir) = std::env::var("APPDIR")
        && let Some(display) = gtk::gdk::Display::default()
    {
        let theme = gtk::IconTheme::for_display(&display);

        if !theme.has_icon("open-menu-symbolic") {
            crate::dev_println!("CLIENT", "Icon not found in system theme. Using fallback.");

            let fallback_path = std::path::Path::new(&appdir).join("icons");
            theme.add_search_path(fallback_path);
        }
    }

    let gui_args = parse_gui_arguments(cmd_line);
    let settings = get_settings();
    // Apply the saved language before any widgets are built with translated text.
    crate::gui_frontend::i18n::set_language(&settings.string("app-language"));
    // Mirrored into a plain flag: the recording call sites run on workers,
    // which cannot touch a `Settings`.
    action_journal::set_enabled(settings.boolean(action_journal::ENABLED_KEY));
    settings.connect_changed(Some(action_journal::ENABLED_KEY), |settings, key| {
        action_journal::set_enabled(settings.boolean(key));
    });
    let search_card: Rc<RefCell<Option<GSteamAppObject>>> = Rc::new(RefCell::new(None));
    let app_id = Rc::new(Cell::new(Option::<u32>::None));
    let app_unlocked_achievements_count = Rc::new(Cell::new(0usize));
    let prefetched_progress: PrefetchedProgress = Rc::new(RefCell::new(None));
    let idle_count: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let achievement_loader = AchievementLoader::new();

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
        app_playtime_value,
        app_metacritic_box,
        _app_sidebar,
        app_achievements_model,
        app_achievement_string_filter,
        app_stat_model,
        app_stat_string_filter,
        app_pane,
        cancel_timed_unlock,
        app_achievements_stack,
    ) = create_app_view(
        app_id.clone(),
        app_unlocked_achievements_count.clone(),
        application,
    );

    // Loading box
    let list_spinner = Spinner::builder().margin_end(5).spinning(true).build();
    let list_spinner_label = Label::builder().label(tr("Loading...").as_str()).build();
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

    let list_disconnected_icon = Image::from_icon_name("dialog-warning-symbolic");
    list_disconnected_icon.set_icon_size(IconSize::Large);
    let list_disconnected_label = Label::builder()
        .label(tr("SamRewritten could not connect to Steam. Is it running?").as_str())
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();
    let list_disconnected_retry = Button::builder()
        .label(tr("Try again").as_str())
        .action_name("app.refresh_app_list")
        .halign(Align::Center)
        .build();
    let list_disconnected_box = Box::builder()
        .spacing(20)
        .valign(Align::Center)
        .halign(Align::Center)
        .orientation(Orientation::Vertical)
        .build();
    list_disconnected_box.append(&list_disconnected_icon);
    list_disconnected_box.append(&list_disconnected_label);
    list_disconnected_box.append(&list_disconnected_retry);

    // Header bar
    let header_bar = HeaderBar::builder().show_title_buttons(true).build();
    let search_entry = SearchEntry::builder()
        .placeholder_text(tr("Name or AppId (Ctrl+K)").as_str())
        .build();
    let back_button = Button::builder()
        .icon_name("go-previous")
        .visible(false)
        .build();
    let (
        context_menu_button,
        _,
        menu_model,
        context_menu_button_loading,
        context_menu_button_loading_progress_label,
        context_menu_button_info_label,
    ) = create_context_menu_button();
    let sidebar_button = ToggleButton::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text(tr("Show or hide the sidebar").as_str())
        .build();
    header_bar.pack_start(&back_button);
    header_bar.pack_start(&sidebar_button);
    header_bar.pack_start(&search_entry);
    header_bar.pack_end(&context_menu_button);
    header_bar.pack_end(&context_menu_button_loading);

    let list_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .build();

    let list_of_apps_or_no_result = Stack::builder()
        .transition_type(StackTransitionType::Crossfade)
        .hexpand(true)
        .build();
    list_of_apps_or_no_result.add_named(&list_scrolled_window, Some("list"));
    list_of_apps_or_no_result.add_named(&app_list_no_result_box, Some("empty"));

    // Inside the "list" page, not beside the stack, so the details page keeps
    // the full width. It stays up over the "empty" child on purpose: a filter
    // that hides everything has to remain undoable.
    let sidebar = Rc::new(build_sidebar(&settings));
    let list_page = build_list_page(
        &settings,
        &sidebar.widget,
        &list_of_apps_or_no_result,
        &sidebar_button,
    );

    // Main application stack component
    let list_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    list_stack.add_named(&list_spinner_box, Some("loading"));
    list_stack.add_named(&list_page, Some("list"));
    list_stack.add_named(&list_disconnected_box, Some("disconnected"));
    list_stack.add_named(&app_pane, Some("app"));

    // App list models
    let list_factory = SignalListItemFactory::new();
    let list_store = ListStore::new::<GSteamAppObject>();
    achievement_loader.attach(&list_store);

    // Hot-path caches — avoid repeated GSettings reads and to_lowercase() allocations
    // inside the filter/sort closures. Updated by their respective change handlers.
    let filter_state: Rc<FilterState> = Rc::new(FilterState::from_settings(&settings));
    let sort_mode_cache: Rc<RefCell<String>> =
        Rc::new(RefCell::new(settings.string("app-sort").to_string()));
    let search_text_lower: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let counts_ready: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let counts_wanted_by_profile: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    let list_custom_filter = gtk::CustomFilter::new(clone!(
        #[strong]
        filter_state,
        #[strong]
        counts_ready,
        #[strong]
        search_text_lower,
        move |obj| {
            let app = obj.downcast_ref::<GSteamAppObject>().unwrap();

            if app.is_synthetic() {
                return true;
            }

            if filter_state.junk.get() && app.is_junk() {
                return false;
            }
            if filter_state.only_idling.get() && !app.is_idling() {
                return false;
            }
            if filter_state.hide_never_launched.get() && app.last_played() == 0 {
                return false;
            }

            if counts_ready.get() {
                let total = app.achievement_count();
                let unlocked = app.unlocked_achievement_count();
                if filter_state.hide_without_achievements.get() && total == 0 {
                    return false;
                }
                if filter_state.hide_fully_unlocked.get() && total > 0 && unlocked >= total {
                    return false;
                }
                if filter_state.hide_no_unlocked.get() && unlocked == 0 {
                    return false;
                }
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
        #[strong]
        counts_ready,
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
                "completion" if counts_ready.get() => b
                    .completion()
                    .partial_cmp(&a.completion())
                    .unwrap_or(Ordering::Equal)
                    .then_with(alphabetical),
                "remaining" if counts_ready.get() => {
                    a.remaining().cmp(&b.remaining()).then_with(alphabetical)
                }
                "completion" | "remaining" => alphabetical(),
                _ => a.app_id().cmp(&b.app_id()),
            };
            ord.into()
        }
    ));

    // `FilterListModel` only signals items-changed when the *filtered* count
    // moves, so a filter that rejects everything never fires at all.
    let sync_empty_state: Rc<dyn Fn()> = Rc::new(clone!(
        #[weak]
        list_filter_model,
        #[weak]
        list_of_apps_or_no_result,
        move || {
            let child = if list_filter_model.n_items() == 0 {
                "empty"
            } else {
                "list"
            };
            if list_of_apps_or_no_result.visible_child_name().as_deref() != Some(child) {
                list_of_apps_or_no_result.set_visible_child_name(child);
            }
        }
    ));

    let sync_counts_state: Rc<dyn Fn(bool)> = Rc::new(clone!(
        #[weak]
        list_store,
        #[weak]
        list_custom_filter,
        #[weak]
        list_custom_sorter,
        #[strong]
        counts_ready,
        #[strong]
        filter_state,
        #[strong]
        sort_mode_cache,
        #[strong]
        sidebar,
        #[strong]
        achievement_loader,
        #[strong]
        counts_wanted_by_profile,
        // Only landed counts can change what a completion filter decides, and
        // the search box mutates the store twice per keystroke.
        move |counts_moved: bool| {
            let needs_counts = filter_state.depends_on_counts()
                || sort_needs_counts(sort_mode_cache.borrow().as_str())
                || counts_wanted_by_profile.get();
            let (loaded, total) = achievement_loader.counts_progress(&list_store);
            let ready = total > 0 && loaded == total;
            if ready {
                counts_wanted_by_profile.set(false);
            }

            // Once measured, every count change re-runs them: unlocking a
            // game's last achievement is what makes it match "hide at 100%".
            // Mid-sweep only the flip counts.
            let readiness_changed = counts_ready.replace(ready) != ready;
            if readiness_changed || (ready && counts_moved) {
                if filter_state.depends_on_counts() {
                    list_custom_filter.changed(gtk::FilterChange::Different);
                }
                if sort_needs_counts(sort_mode_cache.borrow().as_str()) {
                    list_custom_sorter.changed(gtk::SorterChange::Different);
                }
            }

            if !needs_counts || total == 0 {
                achievement_loader.cancel_backlog();
                sidebar.set_counts_loading(false, 0.0);
            } else if ready {
                sidebar.set_counts_loading(false, 1.0);
            } else {
                achievement_loader.queue_remaining(&list_store);
                sidebar.set_counts_loading(true, f64::from(loaded) / f64::from(total));
            }
        }
    ));

    let on_filters_changed: Rc<dyn Fn()> = Rc::new(clone!(
        #[strong]
        sync_counts_state,
        #[strong]
        sync_empty_state,
        move || {
            sync_counts_state(false);
            sync_empty_state();
        }
    ));

    let identity: SharedIdentity = Rc::new(Identity::default());
    let on_open_app: Rc<dyn Fn(&GSteamAppObject)> = Rc::new(clone!(
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
        app_playtime_value,
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
        #[strong]
        app_id,
        #[strong]
        prefetched_progress,
        #[strong]
        settings,
        move |app: &GSteamAppObject| {
            switch_from_app_list_to_app(
                app,
                application,
                &app_type_value,
                &app_developer_value,
                &app_achievement_count_value,
                &app_stats_count_value,
                app_stack,
                &app_id,
                &app_metacritic_box,
                &app_metacritic_value,
                &app_playtime_value,
                &app_shimmer_image,
                &app_label,
                &menu_model,
                &list_stack,
                &prefetched_progress,
                &settings,
            );
        }
    ));
    let on_measure_all: Rc<dyn Fn()> = Rc::new(clone!(
        #[strong]
        counts_wanted_by_profile,
        #[strong]
        sync_counts_state,
        move || {
            counts_wanted_by_profile.set(true);
            sync_counts_state(false);
        }
    ));
    let counts_loading: Rc<dyn Fn() -> bool> = Rc::new(clone!(
        #[strong]
        achievement_loader,
        move || achievement_loader.is_working()
    ));
    let profile = Rc::new(build_profile_view(
        identity.clone(),
        on_open_app,
        on_measure_all,
        counts_loading,
    ));
    list_stack.add_named(&profile.widget, Some("profile"));

    sidebar.connect_profile_clicked(clone!(
        #[weak]
        list_stack,
        #[weak]
        list_store,
        #[strong]
        profile,
        #[weak]
        application,
        #[weak]
        menu_model,
        move || {
            profile.load(&list_store);
            set_context_popover_to_profile_context(&menu_model, &application);
            list_stack.set_visible_child_name("profile");
        }
    ));

    sidebar.connect_counts_load_clicked(clone!(
        #[strong]
        settings,
        #[strong]
        achievement_loader,
        #[strong]
        counts_wanted_by_profile,
        move || {
            achievement_loader.cancel_backlog();
            counts_wanted_by_profile.set(false);

            if sort_needs_counts(settings.string("app-sort").as_str())
                && let Err(e) = settings.set_string("app-sort", "alphabetical")
            {
                eprintln!("[CLIENT] Error saving app-sort setting: {e:?}");
            }
            for key in COUNT_FILTER_KEYS {
                if settings.boolean(key)
                    && let Err(e) = settings.set_boolean(key, false)
                {
                    eprintln!("[CLIENT] Error saving {key} setting: {e:?}");
                }
            }
        }
    ));

    setup_settings_bindings(
        application,
        &settings,
        &list_custom_filter,
        &list_custom_sorter,
        filter_state.clone(),
        sort_mode_cache.clone(),
        on_filters_changed.clone(),
    );

    achievement_loader.on_counts_applied(clone!(
        #[strong]
        sync_counts_state,
        #[strong]
        profile,
        #[weak]
        list_store,
        #[weak]
        list_stack,
        move || {
            sync_counts_state(true);
            if list_stack.visible_child_name().as_deref() == Some("profile") {
                profile.queue_refresh(&list_store);
            }
        }
    ));
    list_store.connect_items_changed(clone!(
        #[strong]
        on_filters_changed,
        move |_, _, _, _| on_filters_changed()
    ));

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

    profile.connect_select_apps(clone!(
        #[weak]
        list_selection_model,
        #[weak]
        list_stack,
        #[weak]
        application,
        #[weak]
        menu_model,
        move |app_ids: &[u32]| {
            let wanted: HashSet<u32> = app_ids.iter().copied().collect();
            list_selection_model.unselect_all();
            for position in 0..list_selection_model.n_items() {
                if let Some(app) = list_selection_model
                    .item(position)
                    .and_downcast::<GSteamAppObject>()
                    && wanted.contains(&app.app_id())
                {
                    list_selection_model.select_item(position, false);
                }
            }

            let has_selection = !list_selection_model.selection().is_empty();
            set_app_action_enabled(&application, "unlock_all_apps", has_selection);
            set_app_action_enabled(&application, "lock_all_apps", has_selection);
            set_app_action_enabled(&application, "export_selected_progress", has_selection);
            set_context_popover_to_app_list_context(&menu_model, &application);
            list_stack.set_visible_child_name("list");
        }
    ));

    profile.connect_undone(clone!(
        #[weak]
        list_store,
        #[strong]
        achievement_loader,
        move |app_ids: &[u32]| {
            for app_id in app_ids {
                achievement_loader.refresh_app(*app_id, &list_store);
            }
        }
    ));

    let window = ApplicationWindow::builder()
        .application(application)
        .title("SamRewritten")
        .default_width(904) // Somehow.. min width with default theme
        .default_height(600)
        .child(&list_stack)
        .titlebar(&header_bar)
        .build();

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
        app_playtime_value,
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
        #[strong]
        prefetched_progress,
        #[strong]
        settings,
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
                &app_playtime_value,
                &app_shimmer_image,
                &app_label,
                &menu_model,
                &list_stack,
                &prefetched_progress,
                &settings,
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
        #[strong]
        idle_count,
        #[strong]
        prefetched_progress,
        #[strong]
        filter_state,
        #[strong]
        sync_empty_state,
        #[weak]
        application,
        #[weak]
        list_custom_filter,
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
        app_playtime_value,
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
        #[strong]
        settings,
        move |_, list_item| {
            let card = SteamAppCard::default();
            card.set_size_request(CARD_MIN_WIDTH, CARD_HEIGHT);
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
                app_playtime_value,
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
                #[strong]
                prefetched_progress,
                #[strong]
                settings,
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
                        &app_playtime_value,
                        &app_shimmer_image,
                        &app_label,
                        &menu_model,
                        &list_stack,
                        &prefetched_progress,
                        &settings,
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
                    match Command::new(get_executable_path())
                        .arg(format!("--auto-open={app_id_to_bind}"))
                        .spawn()
                    {
                        // Without the wait every window opened this way stays a
                        // zombie until this process exits.
                        Ok(mut child) => {
                            std::thread::spawn(move || {
                                let _ = child.wait();
                            });
                        }
                        Err(e) => eprintln!("[CLIENT] Could not open {app_id_to_bind}: {e}"),
                    }
                }
            ));

            card.idle_button().connect_toggled(clone!(
                #[weak]
                card,
                #[weak]
                list_store,
                #[strong]
                idle_count,
                #[weak]
                list_custom_filter,
                #[strong]
                filter_state,
                #[strong]
                sync_empty_state,
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
                    apply_idle_cap_delta(&list_store, &idle_count, if active { 1 } else { -1 });
                    if filter_state.only_idling.get() {
                        list_custom_filter.changed(gtk::FilterChange::Different);
                        sync_empty_state();
                    }

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
                        #[strong]
                        idle_count,
                        #[weak]
                        list_custom_filter,
                        #[strong]
                        filter_state,
                        async move {
                            if let Ok(Err(e)) = handle.await {
                                eprintln!(
                                    "[CLIENT] {} app {app_id} failed: {e:?}",
                                    if active { "Launching" } else { "Stopping" }
                                );
                                app.set_is_idling(!active);
                                apply_idle_cap_delta(
                                    &list_store,
                                    &idle_count,
                                    if active { -1 } else { 1 },
                                );
                                if filter_state.only_idling.get() {
                                    list_custom_filter.changed(gtk::FilterChange::Different);
                                }
                            }
                        }
                    ));
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
                        set_app_action_enabled(&application, "export_selected_progress", true);
                    } else {
                        list_selection_model.unselect_item(position);
                        let selection = list_selection_model.selection();
                        let has_selection = !selection.is_empty();
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
        }
    ));

    list_factory.connect_bind(clone!(
        #[strong]
        achievement_loader,
        #[weak]
        list_store,
        move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be a ListItem");
            let Some(item) = list_item.item() else {
                return;
            };
            let Ok(app) = item.downcast::<GSteamAppObject>() else {
                return;
            };
            if app.achievements_loaded() {
                return;
            }
            achievement_loader.prioritize(app.app_id());
            achievement_loader.kick(&list_store);
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
        #[strong]
        search_card,
        move |entry| {
            let text = Some(entry.text()).filter(|s| !s.is_empty());
            // Refresh the lowercased cache that the list filter reads on every item.
            *search_text_lower.borrow_mut() =
                text.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();

            let previous = search_card
                .borrow()
                .as_ref()
                .and_then(|card| list_store.find(card));

            if let Some(app_id) = text.as_ref().and_then(|t| t.parse::<u32>().ok()) {
                let synthetic = GSteamAppObject::new(AppModel {
                    app_id,
                    app_name: format!("App {app_id}"),
                    app_type: AppModelType::App,
                    developer: "Unknown".to_string(),
                    image_url: None,
                    metacritic_score: None,
                    playtime_minutes: None,
                    last_played: None,
                    achievement_count: None,
                    unlocked_achievement_count: None,
                });
                synthetic.set_is_synthetic(true);
                list_store.insert(previous.map_or(0, |at| at + 1), &synthetic);
                *search_card.borrow_mut() = Some(synthetic);
            } else {
                *search_card.borrow_mut() = None;
            }

            app_achievement_string_filter.set_search(text.as_deref());
            app_stat_string_filter.set_search(text.as_deref());
            list_custom_filter.changed(gtk::FilterChange::Different);

            if let Some(at) = previous {
                list_store.remove(at);
            }
        }
    ));

    list_filter_model.connect_items_changed(clone!(
        #[strong]
        sync_empty_state,
        move |_, _, _, _| sync_empty_state()
    ));

    // Back button handler
    back_button.connect_clicked(clone!(
        #[weak]
        list_stack,
        #[weak]
        app_id,
        #[weak]
        list_store,
        #[weak]
        menu_model,
        #[weak]
        application,
        #[weak]
        app_achievements_model,
        #[weak]
        app_stat_model,
        #[strong]
        achievement_loader,
        #[strong]
        cancel_timed_unlock,
        move |_| {
            if list_stack.visible_child_name().as_deref() == Some("profile") {
                set_context_popover_to_app_list_context(&menu_model, &application);
                list_stack.set_visible_child_name("list");
                return;
            }

            cancel_timed_unlock.store(true, std::sync::atomic::Ordering::Relaxed);
            list_stack.set_visible_child_name("list");
            // The one way out of a timed unlock that skips the refresh, so it
            // has to lift the lockout itself.
            set_timed_unlock_actions_enabled(&application, true);
            set_context_popover_to_app_list_context(&menu_model, &application);
            application.activate_action("app_page_closed", None);
            if let Some(app_id) = app_id.take() {
                achievement_loader.refresh_app(app_id, &list_store);
                spawn_blocking(move || {
                    let _ = StopApp { app_id }.request();
                });
            }

            // Clear achievements and stats for performance, but wait a bit before doing so
            // to avoid flashes of the data disappearing during the animation
            let handle = spawn_blocking(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
            });

            MainContext::default().spawn_local(clone!(
                #[strong]
                app_id,
                async move {
                    if Some(()) != handle.await.ok() {
                        eprintln!("[CLIENT] Threading task failed");
                    }

                    // An app opened during the wait already put its rows here.
                    if app_id.get().is_some() {
                        return;
                    }

                    app_achievements_model.remove_all();
                    app_stat_model.remove_all();
                }
            ));
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
        &list_store,
        achievement_loader.clone(),
        &context_menu_button,
        &context_menu_button_loading,
        &context_menu_button_loading_progress_label,
        &context_menu_button_info_label,
    );

    let (action_export_selected, action_import_progress) = create_progress_actions(
        application,
        &grid_view,
        &list_store,
        achievement_loader.clone(),
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
        idle_count.clone(),
        achievement_loader.clone(),
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
        &app_achievements_stack,
        &cancel_timed_unlock,
        &prefetched_progress,
        &settings,
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
        sidebar_button,
        #[weak]
        application,
        #[weak]
        app_stack,
        #[weak]
        search_entry,
        #[weak]
        action_refresh_app_list,
        #[strong]
        prefetched_progress,
        #[strong]
        settings,
        move |stack| {
            let page = stack.visible_child_name();
            let page = page.as_deref();
            let on_own_page = page == Some("app") || page == Some("profile");
            sidebar_button.set_visible(!on_own_page);
            sidebar_button.set_sensitive(page == Some("list"));
            search_entry.set_sensitive(page != Some("profile"));

            if page == Some("loading") {
                back_button.set_visible(false);
                action_refresh_app_list.set_enabled(false);
            } else if page == Some("profile") {
                search_entry.set_text("");
                back_button.set_visible(true);
                action_refresh_app_list.set_enabled(false);
            } else if page == Some("app") {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some(tr("Achievement or stat...").as_str()));
                back_button.set_visible(true);
                action_refresh_app_list.set_enabled(false);
            } else {
                search_entry.set_text("");
                search_entry.set_placeholder_text(Some(tr("Name or AppId (Ctrl+K)").as_str()));
                back_button.set_visible(false);
                action_refresh_app_list.set_enabled(true);

                let auto_launch_app = gui_args.auto_open.get();
                if auto_launch_app > 0 && page == Some("list") {
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
                            achievement_count: None,
                            unlocked_achievement_count: None,
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
                        &app_playtime_value,
                        &app_shimmer_image,
                        &app_label,
                        &menu_model,
                        stack,
                        &prefetched_progress,
                        &settings,
                    );
                }
            }
        }
    ));

    setup_app_actions(
        application,
        &action_refresh_app_list,
        &action_refresh_achievements_list,
        &action_clear_all_stats_and_achievements,
        &action_select_all_apps,
        &action_unselect_all_apps,
        &action_unlock_all_selected,
        &action_lock_all_selected,
        &action_export_selected,
        &action_import_progress,
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

    #[cfg(debug_assertions)]
    crate::gui_frontend::dev_widgets::install(&window);

    choose_steam_install_then(
        &window,
        clone!(
            #[strong]
            app_stack,
            #[strong]
            list_stack,
            #[strong]
            action_refresh_app_list,
            #[strong]
            window,
            move |chosen| {
                #[cfg(unix)]
                if let Some(root) = chosen.as_ref() {
                    crate::utils::snap::pin_install_root(root);
                }
                if let Err(e) = crate::backend::orchestrator_client::spawn_orchestrator(chosen) {
                    eprintln!("[CLIENT] Failed to start orchestrator: {e}");
                }
                app_stack.set_visible_child_name("loading");
                list_stack.set_visible_child_name("loading");
                action_refresh_app_list.activate(None);
                action_refresh_app_list.set_enabled(false);
                load_identity(
                    identity.clone(),
                    clone!(
                        #[strong]
                        sidebar,
                        #[strong]
                        profile,
                        move |id| {
                            sidebar.set_identity(id);
                            profile.refresh_identity();
                        }
                    ),
                );
                window.present();
            }
        ),
    );

    ExitCode::SUCCESS
}
