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

use crate::gui_frontend::MainApplication;
use crate::gui_frontend::custom_progress_bar_widget::CustomProgressBar;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::glib::clone;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, ClosureExpression, Frame, Label, ListBox, ListBoxRow, ListView,
    NoSelection, Orientation, ScrolledWindow, SelectionMode, SignalListItemFactory, Stack,
    StackTransitionType, Widget, glib,
};

#[inline]
fn create_header(application: &MainApplication) -> (ListBox, Button) {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .build();
    let hbox = Box::new(Orientation::Horizontal, 10);
    hbox.set_spacing(5);

    let button_stop = Button::builder().icon_name("go-previous").build();
    let label = Label::builder().label("Stop and go back").build();

    hbox.append(&button_stop);
    hbox.append(&label);

    button_stop.connect_clicked(clone!(
        #[weak]
        application,
        move |_| {
            application.activate_action("refresh_achievements_list", None);
        }
    ));

    let list_box_row = ListBoxRow::builder()
        .child(&hbox)
        .margin_end(5)
        .margin_start(5)
        .margin_top(5)
        .margin_bottom(5)
        .activatable(false)
        .focusable(false)
        .build();
    list.append(&list_box_row);

    (list, button_stop)
}

#[inline]
pub fn create_achievements_automatic_view(
    timed_filtered_model: &NoSelection,
    application: &MainApplication,
) -> (Frame, Button) {
    let (header, header_achievements_stop) = create_header(application);

    let achievements_list_factory = SignalListItemFactory::new();

    let app_achievements_list_view = ListView::builder()
        .orientation(Orientation::Vertical)
        .model(timed_filtered_model)
        .factory(&achievements_list_factory)
        .build();
    let app_achievements_scrolled_window = ScrolledWindow::builder()
        .child(&app_achievements_list_view)
        .vexpand(true)
        .build();

    achievements_list_factory.connect_setup(move |_, list_item| {
        let normal_icon = ShimmerImage::new();
        normal_icon.set_size_request(32, 32);
        let locked_icon = ShimmerImage::new();
        locked_icon.set_size_request(32, 32);

        let icon_stack = Stack::builder()
            .transition_type(StackTransitionType::RotateLeftRight)
            .build();
        icon_stack.add_named(&normal_icon, Some("normal"));
        icon_stack.add_named(&locked_icon, Some("locked"));

        let icon_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Start)
            .margin_end(8)
            .build();
        icon_box.append(&icon_stack);

        let spacer = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let name_label = Label::builder()
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let description_label = Label::builder()
            .ellipsize(EllipsizeMode::End)
            .halign(Align::Start)
            .build();
        let remaining_time_label = Label::builder().build();
        let label_box = Box::builder().orientation(Orientation::Vertical).build();
        let global_percentage_progress_bar = CustomProgressBar::new();
        global_percentage_progress_bar.set_height_request(2);
        label_box.append(&name_label);
        label_box.append(&description_label);
        let entry_box = Box::builder().orientation(Orientation::Vertical).build();
        let achievement_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        achievement_box.append(&icon_box);
        achievement_box.append(&label_box);
        achievement_box.append(&spacer);
        achievement_box.append(&remaining_time_label);
        entry_box.append(&achievement_box);
        entry_box.append(&global_percentage_progress_bar);
        list_item.set_child(Some(&entry_box));

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("name")
            .bind(&name_label, "label", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("description")
            .bind(&description_label, "label", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-normal")
            .bind(&normal_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("icon-locked")
            .bind(&locked_icon, "url", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent")
            .bind(&global_percentage_progress_bar, "value", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("global-achieved-percent-ok")
            .bind(&global_percentage_progress_bar, "visible", Widget::NONE);

        list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("time-until-unlock")
            .bind(&remaining_time_label, "label", Widget::NONE);

        // Custom expressions
        let is_achieved_expr = list_item
            .property_expression("item")
            .chain_property::<GAchievementObject>("is-achieved");

        let achieved_visible_icon_closure = glib::RustClosure::new(|values: &[glib::Value]| {
            let is_achieved = values
                .get(1)
                .and_then(|val| val.get::<bool>().ok())
                .unwrap_or(false);
            let child_name = if is_achieved { "normal" } else { "locked" };
            Some(child_name.to_value())
        });

        let visible_child_expr =
            ClosureExpression::new::<String>(&[is_achieved_expr], achieved_visible_icon_closure);

        visible_child_expr.bind(&icon_stack, "visible-child-name", Widget::NONE);
    });

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&header);
    vbox.append(&app_achievements_scrolled_window);
    let app_achievements_frame = Frame::builder()
        .margin_end(15)
        .margin_start(15)
        .margin_top(15)
        .margin_bottom(15)
        .child(&vbox)
        .build();

    (app_achievements_frame, header_achievements_stop)
}
