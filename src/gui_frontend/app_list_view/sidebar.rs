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

//! The app list's left sidebar: profile card, library filters, sort order. The
//! widgets are views onto GSettings keys; `settings_bindings` re-runs the
//! filter and sorter when they change.

use crate::gui_frontend::i18n::{tr, tr_noop};
use crate::gui_frontend::profile_view::identity::Identity;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use gtk::gio::Settings;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, CheckButton, Label, Orientation, PolicyType, ProgressBar, ScrolledWindow,
    Separator, Spinner,
};

pub(super) const SIDEBAR_WIDTH: i32 = 232;
const AVATAR_SIZE: i32 = 48;

struct FilterSpec {
    key: &'static str,
    label: &'static str,
    /// The checkbox shows the negation of the key. Only `filter-junk` uses it:
    /// junk is hidden by default, so a `Hide junk` box would sit permanently
    /// ticked for everyone.
    invert: bool,
}

const FILTERS: &[FilterSpec] = &[
    FilterSpec {
        key: "filter-hide-without-achievements",
        label: tr_noop("Hide with no achievements"),
        invert: false,
    },
    FilterSpec {
        key: "filter-hide-fully-unlocked",
        label: tr_noop("Hide at 100%"),
        invert: false,
    },
    FilterSpec {
        key: "filter-hide-no-unlocked",
        label: tr_noop("Hide at 0%"),
        invert: false,
    },
    FilterSpec {
        key: "filter-hide-never-launched",
        label: tr_noop("Hide never launched"),
        invert: false,
    },
    FilterSpec {
        key: "filter-only-idling",
        label: tr_noop("Only currently idling"),
        invert: false,
    },
    FilterSpec {
        key: "filter-junk",
        label: tr_noop("Show junk"),
        invert: true,
    },
];

struct SortSpec {
    value: &'static str,
    label: &'static str,
    needs_counts: bool,
}

const SORT_MODES: &[SortSpec] = &[
    SortSpec {
        value: "app_id",
        label: tr_noop("App ID"),
        needs_counts: false,
    },
    SortSpec {
        value: "alphabetical",
        label: tr_noop("Name"),
        needs_counts: false,
    },
    SortSpec {
        value: "last_played",
        label: tr_noop("Last played"),
        needs_counts: false,
    },
    SortSpec {
        value: "playtime",
        label: tr_noop("Playtime"),
        needs_counts: false,
    },
    SortSpec {
        value: "completion",
        label: tr_noop("Completion"),
        needs_counts: true,
    },
    SortSpec {
        value: "remaining",
        label: tr_noop("Achievements left"),
        needs_counts: true,
    },
];

pub(super) fn sort_needs_counts(value: &str) -> bool {
    SORT_MODES
        .iter()
        .any(|spec| spec.value == value && spec.needs_counts)
}

pub(super) struct Sidebar {
    pub widget: Box,
    avatar: ShimmerImage,
    name_label: Label,
    profile_button: Button,
    loading_button: Button,
    loading_spinner: Spinner,
    loading_progress: ProgressBar,
}

fn section_label(text: &str) -> Label {
    Label::builder()
        .label(text)
        .xalign(0.0)
        .margin_top(6)
        .css_classes(["heading", "dim-label"])
        .build()
}

fn build_profile_card(avatar: &ShimmerImage, name: &Label) -> Button {
    avatar.set_size_request(AVATAR_SIZE, AVATAR_SIZE);
    avatar.set_valign(Align::Center);

    let text_box = Box::builder()
        .orientation(Orientation::Vertical)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    text_box.append(name);
    text_box.append(
        &Label::builder()
            .label(tr("View profile").as_str())
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let content = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    content.append(avatar);
    content.append(&text_box);

    Button::builder()
        .css_classes(["flat"])
        .margin_top(6)
        .margin_bottom(6)
        .child(&content)
        .build()
}

fn wire_check(settings: &Settings, spec: &'static FilterSpec, check: &CheckButton) {
    let shown = |value: bool| if spec.invert { !value } else { value };

    check.set_active(shown(settings.boolean(spec.key)));
    check.connect_toggled(clone!(
        #[strong]
        settings,
        move |check| {
            let value = shown(check.is_active());
            if settings.boolean(spec.key) != value
                && let Err(e) = settings.set_boolean(spec.key, value)
            {
                eprintln!("[CLIENT] Error saving {} setting: {e:?}", spec.key);
            }
        }
    ));
    settings.connect_changed(
        Some(spec.key),
        clone!(
            #[weak]
            check,
            move |s, _| {
                let active = shown(s.boolean(spec.key));
                if check.is_active() != active {
                    check.set_active(active);
                }
            }
        ),
    );
}

pub(super) fn build_sidebar(settings: &Settings) -> Sidebar {
    let avatar = ShimmerImage::new();
    let name_label = Label::builder()
        .label(tr("Steam user").as_str())
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["heading"])
        .build();

    let content = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let profile_button = build_profile_card(&avatar, &name_label);
    content.append(&profile_button);
    content.append(&Separator::new(Orientation::Horizontal));

    let loading_spinner = Spinner::builder().valign(Align::Center).build();
    let loading_title = Label::builder()
        .label(tr("Fetching completion…").as_str())
        .xalign(0.0)
        .wrap(true)
        .css_classes(["caption"])
        .build();
    let loading_subtitle = Label::builder()
        .label(tr("Click to cancel").as_str())
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let loading_progress = ProgressBar::builder()
        .valign(Align::Center)
        .margin_top(3)
        .margin_bottom(3)
        .build();
    let loading_text = Box::builder()
        .orientation(Orientation::Vertical)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    loading_text.append(&loading_title);
    loading_text.append(&loading_progress);
    loading_text.append(&loading_subtitle);
    let loading_content = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    loading_content.append(&loading_spinner);
    loading_content.append(&loading_text);
    let loading_button = Button::builder()
        .margin_top(6)
        .css_classes(["flat"])
        .child(&loading_content)
        .build();
    content.append(&loading_button);

    content.append(&section_label(tr("Filters").as_str()));
    for spec in FILTERS {
        let check = CheckButton::with_label(tr(spec.label).as_str());
        wire_check(settings, spec, &check);
        content.append(&check);
    }

    content.append(&Separator::new(Orientation::Horizontal));
    content.append(&section_label(tr("Sort by").as_str()));

    // Grouped radios cannot be bound the way the checkboxes are: write on
    // toggle, read back on change.
    let current = settings.string("app-sort");
    let mut first: Option<CheckButton> = None;
    let mut radios: Vec<(&str, CheckButton)> = Vec::with_capacity(SORT_MODES.len());
    for spec in SORT_MODES {
        let radio = CheckButton::with_label(tr(spec.label).as_str());
        match first {
            Some(ref group) => radio.set_group(Some(group)),
            None => first = Some(radio.clone()),
        }
        radio.set_active(current == spec.value);
        radio.connect_toggled(clone!(
            #[strong]
            settings,
            move |radio| {
                if radio.is_active()
                    && settings.string("app-sort") != spec.value
                    && let Err(e) = settings.set_string("app-sort", spec.value)
                {
                    eprintln!("[CLIENT] Error saving app-sort setting: {e:?}");
                }
            }
        ));
        content.append(&radio);
        radios.push((spec.value, radio));
    }
    settings.connect_changed(Some("app-sort"), move |s, _| {
        let value = s.string("app-sort");
        for (mode, radio) in &radios {
            if (value == *mode) != radio.is_active() {
                radio.set_active(value == *mode);
            }
        }
    });

    let reset_button = Button::builder()
        .label(tr("Reset filters").as_str())
        .halign(Align::Fill)
        .margin_top(12)
        .build();
    reset_button.connect_clicked(clone!(
        #[strong]
        settings,
        move |_| {
            for spec in FILTERS {
                settings.reset(spec.key);
            }
        }
    ));
    content.append(&reset_button);

    let scroller = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .propagate_natural_height(true)
        .vexpand(true)
        .child(&content)
        .build();

    // hexpand is pinned off on purpose: the profile card's text column sets it,
    // and it would propagate up, letting the sidebar take half the window.
    let widget = Box::builder()
        .orientation(Orientation::Horizontal)
        .width_request(SIDEBAR_WIDTH)
        .hexpand(false)
        .css_classes(["view"])
        .build();
    widget.append(&scroller);
    widget.append(&Separator::new(Orientation::Vertical));

    let sidebar = Sidebar {
        widget,
        avatar,
        name_label,
        profile_button,
        loading_button,
        loading_spinner,
        loading_progress,
    };
    sidebar.set_counts_loading(false, 0.0);
    sidebar
}

impl Sidebar {
    pub(super) fn set_counts_loading(&self, loading: bool, fraction: f64) {
        self.loading_button.set_visible(loading);
        if loading {
            self.loading_spinner.start();
            self.loading_progress.set_fraction(fraction.clamp(0.0, 1.0));
        } else {
            self.loading_spinner.stop();
        }
    }

    pub(super) fn connect_counts_load_clicked(&self, f: impl Fn() + 'static) {
        self.loading_button.connect_clicked(move |_| f());
    }

    pub(super) fn connect_profile_clicked(&self, f: impl Fn() + 'static) {
        self.profile_button.connect_clicked(move |_| f());
    }

    pub(super) fn set_identity(&self, identity: &Identity) {
        let persona = identity.persona.borrow();
        if !persona.is_empty() {
            self.name_label.set_label(&persona);
        }
        if let Some(image) = identity.avatar.borrow().as_ref() {
            self.avatar
                .set_rgba(image.width as i32, image.height as i32, &image.rgba);
        }
    }
}
