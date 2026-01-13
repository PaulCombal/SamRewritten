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

use super::stat_view::create_stats_view;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::achievement_view::create_achievements_view;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::gio::ListStore;
use gtk::glib::clone;
use gtk::pango::{EllipsizeMode, WrapMode};
use gtk::prelude::*;
use gtk::{Align, Box, Frame, Label, Orientation, Separator, Spinner, Stack, StackTransitionType, StringFilter, ToggleButton};
use gtk::{Paned, glib};
use std::cell::Cell;
use std::rc::Rc;

pub fn create_app_view(
    app_id: Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: Rc<Cell<usize>>,
    application: &MainApplication,
) -> (
    Stack,
    ShimmerImage,
    Label,
    ToggleButton,
    ToggleButton,
    Label,
    Label,
    Label,
    Label,
    Label,
    Box,
    Box,
    ListStore,
    StringFilter,
    ListStore,
    StringFilter,
    Paned,
    Frame,
) {
    let app_spinner = Spinner::builder().spinning(true).margin_end(5).build();
    let app_spinner_label = Label::builder().label("Loading...").build();
    let app_spinner_box = Box::builder().halign(Align::Center).build();
    app_spinner_box.append(&app_spinner);
    app_spinner_box.append(&app_spinner_label);

    let app_achievement_count_label = Label::builder()
        .label("Achievements:")
        .halign(Align::Start)
        .build();
    let app_achievement_count_spacer = Box::builder().hexpand(true).build();
    let app_achievement_count_value = Label::builder().halign(Align::End).build();
    let app_achievement_count_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_achievement_count_box.append(&app_achievement_count_label);
    app_achievement_count_box.append(&app_achievement_count_spacer);
    app_achievement_count_box.append(&app_achievement_count_value);

    let app_stats_count_label = Label::builder()
        .label("Stats:")
        .halign(Align::Start)
        .build();
    let app_stats_count_spacer = Box::builder().hexpand(true).build();
    let app_stats_count_value = Label::builder().halign(Align::End).build();
    let app_stats_count_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_stats_count_box.append(&app_stats_count_label);
    app_stats_count_box.append(&app_stats_count_spacer);
    app_stats_count_box.append(&app_stats_count_value);

    let app_type_label = Label::builder().label("Type:").halign(Align::Start).build();
    let app_type_spacer = Box::builder().hexpand(true).build();
    let app_type_value = Label::builder().halign(Align::End).build();
    let app_type_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_type_box.append(&app_type_label);
    app_type_box.append(&app_type_spacer);
    app_type_box.append(&app_type_value);

    let app_developer_label = Label::builder()
        .label("Developer:")
        .halign(Align::Start)
        .build();
    let app_developer_spacer = Box::builder().hexpand(true).build();
    let app_developer_value = Label::builder()
        .halign(Align::End)
        .ellipsize(EllipsizeMode::End)
        .build();
    let app_developer_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(20)
        .build();
    app_developer_box.append(&app_developer_label);
    app_developer_box.append(&app_developer_spacer);
    app_developer_box.append(&app_developer_value);

    let app_metacritic_label = Label::builder()
        .label("Metacritic:")
        .halign(Align::Start)
        .build();
    let app_metacritic_spacer = Box::builder().hexpand(true).build();
    let app_metacritic_value = Label::builder().halign(Align::End).build();
    let app_metacritic_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(10)
        .build();
    app_metacritic_box.append(&app_metacritic_label);
    app_metacritic_box.append(&app_metacritic_spacer);
    app_metacritic_box.append(&app_metacritic_value);

    let app_loading_failed_label = Label::builder()
        .label("Failed to load app.")
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let app_no_entries_value = Label::builder()
        .label("No entries found.")
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let app_label = Label::builder()
        .margin_top(20)
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(Align::Start)
        .build();

    let app_shimmer_image = ShimmerImage::new();
    app_shimmer_image.set_halign(Align::Fill);
    app_shimmer_image.set_height_request(87);

    let app_achievements_button = ToggleButton::builder().label("Achievements").build();
    let app_stats_button = ToggleButton::builder()
        .label("Stats")
        .group(&app_achievements_button)
        .build();
    let app_button_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"].as_slice())
        .margin_bottom(20)
        .margin_start(0)
        .homogeneous(true)
        .margin_end(0)
        .width_request(231)
        .halign(Align::Start)
        .build();
    app_button_box.append(&app_achievements_button);
    app_button_box.append(&app_stats_button);

    let app_sidebar_separator = Separator::builder()
        .orientation(Orientation::Horizontal)
        .margin_top(20)
        .build();

    let app_sidebar = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();
    app_sidebar.append(&app_button_box);
    app_sidebar.append(&app_shimmer_image);
    app_sidebar.append(&app_label);
    app_sidebar.append(&app_sidebar_separator);
    app_sidebar.append(&app_developer_box);
    app_sidebar.append(&app_metacritic_box);
    app_sidebar.append(&app_achievement_count_box);
    app_sidebar.append(&app_stats_count_box);
    app_sidebar.append(&app_type_box);

    let (
        app_achievements_frame,
        app_achievements_model,
        app_achievement_string_filter,
    ) = create_achievements_view(
        app_id.clone(),
        app_unlocked_achievements_count,
        application,
        &app_achievement_count_value,
    );

    let (app_stat_scrolled_window, app_stat_model, app_stat_string_filter) = create_stats_view();

    let app_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    app_stack.add_named(&app_achievements_frame, Some("achievements"));
    app_stack.add_named(&app_stat_scrolled_window, Some("stats"));
    app_stack.add_named(&app_loading_failed_label, Some("failed"));
    app_stack.add_named(&app_no_entries_value, Some("empty"));
    app_stack.add_named(&app_spinner_box, Some("loading"));

    app_stack.connect_visible_child_name_notify(clone!(
        #[weak]
        app_achievements_button,
        #[weak]
        app_stats_button,
        move |stack| {
            if stack.visible_child_name().as_deref() == Some("loading") {
                app_achievements_button.set_sensitive(false);
                app_stats_button.set_sensitive(false);
            } else if stack.visible_child_name().as_deref() == Some("failed") {
                app_achievements_button.set_sensitive(false);
                app_stats_button.set_sensitive(false);
            } else if stack.visible_child_name().as_deref() == Some("achievements") {
                app_achievements_button.set_active(true);
                app_stats_button.set_active(false);
                app_achievements_button.set_sensitive(true);
                app_stats_button.set_sensitive(true);
            } else {
                app_achievements_button.set_active(false);
                app_stats_button.set_active(true);
                app_achievements_button.set_sensitive(true);
                app_stats_button.set_sensitive(true);
            }
        }
    ));

    app_achievements_button.connect_clicked(clone!(
        #[weak]
        app_stack,
        #[weak]
        app_achievements_model,
        move |_| {
            if app_achievements_model.n_items() == 0 {
                app_stack.set_visible_child_name("empty");
            } else {
                app_stack.set_visible_child_name("achievements");
            }
        }
    ));

    app_stats_button.connect_clicked(clone!(
        #[weak]
        app_stack,
        #[weak]
        app_stat_model,
        move |_| {
            if app_stat_model.n_items() == 0 {
                app_stack.set_visible_child_name("empty");
            } else {
                app_stack.set_visible_child_name("stats");
            }
        }
    ));

    // Create app pane with sidebar and main content
    let app_pane = Paned::builder()
        .orientation(Orientation::Horizontal)
        .shrink_start_child(false)
        .shrink_end_child(false)
        .resize_start_child(false)
        .start_child(&app_sidebar)
        .end_child(&app_stack)
        .build();

    // Return relevant widgets that need to be accessed from outside
    (
        app_stack,
        app_shimmer_image,
        app_label,
        app_achievements_button,
        app_stats_button,
        app_achievement_count_value,
        app_stats_count_value,
        app_type_value,
        app_developer_value,
        app_metacritic_value,
        app_metacritic_box,
        app_sidebar,
        app_achievements_model,
        app_achievement_string_filter,
        app_stat_model,
        app_stat_string_filter,
        app_pane,
        app_achievements_frame,
    )
}
