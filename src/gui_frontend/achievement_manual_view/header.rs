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

use super::copy_controls::CopyControls;
use crate::gui_frontend::gobjects::mode_state::{
    GUnlockModeState, MODE_AUTOCOMMIT, MODE_COPY_TIMING, MODE_DEFERRED,
};
use crate::gui_frontend::i18n::tr;
use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, Label, ListBox, ListBoxRow, Orientation, Popover, SelectionMode,
    ToggleButton,
};
use std::rc::Rc;

pub(super) struct Header {
    pub(super) container: ListBox,
    pub(super) start_button: Button,
    pub(super) queue_label: Label,
    pub(super) auto_fill_button: Button,
}

#[inline]
pub(super) fn create_header(
    mode_state: &Rc<GUnlockModeState>,
    config_popover: &Popover,
    copy: &CopyControls,
) -> Header {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .build();
    let hbox = Box::new(Orientation::Horizontal, 10);

    // Three mutually-exclusive modes as a linked segmented control.
    let instant_toggle = ToggleButton::builder()
        .label(tr("Instant").as_str())
        .build();
    let stage_toggle = ToggleButton::builder()
        .label(tr("Stage").as_str())
        .group(&instant_toggle)
        .build();
    let copy_toggle = ToggleButton::builder()
        .label(tr("Copy user").as_str())
        .group(&instant_toggle)
        .build();
    let mode_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"])
        .valign(Align::Center)
        .build();
    mode_box.append(&instant_toggle);
    mode_box.append(&stage_toggle);
    mode_box.append(&copy_toggle);

    let spacer = Box::builder()
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .build();

    let queue_label = Label::builder().build();
    let auto_fill_button = Button::builder().label(tr("Auto-fill").as_str()).build();
    let auto_fill_dropdown = Button::builder()
        .icon_name("pan-down-symbolic")
        .tooltip_text(tr("Configuration").as_str())
        .build();
    // Anchor the popover to the dropdown button and drive it manually so both
    // halves of the linked group are plain `Button` widgets (same CSS name,
    // identical styling under .linked across themes).
    config_popover.set_parent(&auto_fill_dropdown);
    auto_fill_dropdown.connect_clicked(clone!(
        #[weak]
        config_popover,
        move |_| config_popover.popup()
    ));
    // Manually-parented popovers must be explicitly unparented before the
    // anchor widget is finalized, otherwise GTK warns about lingering children.
    auto_fill_dropdown.connect_destroy(clone!(
        #[strong]
        config_popover,
        move |_| config_popover.unparent()
    ));
    let auto_fill_group = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"])
        .build();
    auto_fill_group.append(&auto_fill_button);
    auto_fill_group.append(&auto_fill_dropdown);

    let start_button = Button::builder()
        .label(tr("Start").as_str())
        .css_classes(["suggested-action"])
        .build();

    let instant_label = Label::builder()
        .label(tr("Changes apply instantly").as_str())
        .css_classes(["dim-label"])
        .build();

    hbox.append(&mode_box);
    hbox.append(&spacer);
    hbox.append(&instant_label);
    hbox.append(&queue_label);
    hbox.append(&auto_fill_group);
    hbox.append(&start_button);
    hbox.append(&copy.avatar_button);
    hbox.append(&copy.advanced_button);
    hbox.append(&copy.start_button);

    match mode_state.mode().as_str() {
        MODE_DEFERRED => stage_toggle.set_active(true),
        MODE_COPY_TIMING => copy_toggle.set_active(true),
        _ => instant_toggle.set_active(true),
    }
    instant_toggle.connect_toggled(clone!(
        #[strong]
        mode_state,
        move |b| {
            if b.is_active() {
                mode_state.set_mode(MODE_AUTOCOMMIT);
            }
        }
    ));
    stage_toggle.connect_toggled(clone!(
        #[strong]
        mode_state,
        move |b| {
            if b.is_active() {
                mode_state.set_mode(MODE_DEFERRED);
            }
        }
    ));
    copy_toggle.connect_toggled(clone!(
        #[strong]
        mode_state,
        move |b| {
            if b.is_active() {
                mode_state.set_mode(MODE_COPY_TIMING);
            }
        }
    ));
    mode_state.connect_mode_notify(clone!(
        #[weak]
        instant_toggle,
        #[weak]
        stage_toggle,
        #[weak]
        copy_toggle,
        move |state| {
            let target = match state.mode().as_str() {
                MODE_DEFERRED => &stage_toggle,
                MODE_COPY_TIMING => &copy_toggle,
                _ => &instant_toggle,
            };
            if !target.is_active() {
                target.set_active(true);
            }
        }
    ));

    // Each mode reveals only its own controls.
    let visibility_apply = clone!(
        #[weak]
        start_button,
        #[weak]
        queue_label,
        #[weak]
        auto_fill_group,
        #[weak(rename_to = copy_avatar)]
        copy.avatar_button,
        #[weak(rename_to = copy_advanced)]
        copy.advanced_button,
        #[weak(rename_to = copy_start)]
        copy.start_button,
        #[weak]
        instant_label,
        move |state: &GUnlockModeState| {
            let deferred = state.mode() == MODE_DEFERRED;
            let copying = state.mode() == MODE_COPY_TIMING;
            instant_label.set_visible(state.mode() == MODE_AUTOCOMMIT);
            start_button.set_visible(deferred);
            queue_label.set_visible(deferred);
            auto_fill_group.set_visible(deferred);
            copy_avatar.set_visible(copying);
            copy_advanced.set_visible(copying);
            copy_start.set_visible(copying);
        }
    );
    visibility_apply(mode_state);
    mode_state.connect_mode_notify(move |state| visibility_apply(state));

    let row = ListBoxRow::builder()
        .child(&hbox)
        .activatable(false)
        .margin_end(5)
        .margin_start(5)
        .margin_top(5)
        .margin_bottom(5)
        .build();
    list.append(&row);

    Header {
        container: list,
        start_button,
        queue_label,
        auto_fill_button,
    }
}
