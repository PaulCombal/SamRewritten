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

//! What SamRewritten changed, and the way back.
//!
//! An undo does not go through `SetAchievement`: that is only forwarded to a
//! child already holding the app open, and nothing is open from here. It goes
//! through `ImportApps`, carrying the recorded before-values as a partial
//! export.

use super::{boxed_list, caption, clear_list, section_heading};
use crate::gui_frontend::dialogs::{confirm_dialog, show_list_dialog};
use crate::gui_frontend::gsettings::get_settings;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{ImportApps, Request};
use crate::utils::action_journal::{self, Batch, Change, Op, Operation, RecordedChange, Reverses};
use crate::utils::ipc_types::{AppAchievementExport, AppExport, AppStatExport, AppStatValue};
use gtk::gio::{Settings, spawn_blocking};
// `glib` itself: `clone!`'s weak captures expand to paths rooted at it.
use gtk::glib;
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, Expander, Label, ListBox, ListBoxRow, MenuButton, Orientation, Switch,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Every row is real widgets on the main loop, so a whole history at once would
/// freeze it for the best part of a second.
const FIRST_ROWS: usize = 25;
const MORE_ROWS: usize = 100;

const MAX_CHANGES: usize = 25;

type OnUndone = Rc<RefCell<Option<Rc<dyn Fn(&[u32])>>>>;

pub(super) struct JournalSection {
    pub widget: Box,
    list: ListBox,
    empty: Label,
    more: Box,
    more_label: Label,
    more_button: Button,
    operations: RefCell<Vec<Operation>>,
    shown: Cell<usize>,
    expanded: RefCell<std::collections::HashSet<u64>>,
    switch: Switch,
    overflow: MenuButton,
    _settings: Settings,
    app_names: RefCell<HashMap<u32, String>>,
    on_undone: OnUndone,
    generation: Rc<Cell<u64>>,
    busy: Cell<bool>,
}

pub(super) fn build_journal_section() -> Rc<JournalSection> {
    let (heading, _spinner) = section_heading(tr("What SamRewritten changed").as_str());
    let switch = Switch::builder()
        .valign(Align::Center)
        .tooltip_text(tr("Keep a history of what SamRewritten changes").as_str())
        .build();
    let settings = get_settings();
    settings
        .bind(action_journal::ENABLED_KEY, &switch, "active")
        .build();
    let menu = gtk::gio::Menu::new();
    menu.append(
        Some(tr("Delete this history").as_str()),
        Some("journal.clear"),
    );
    let overflow = MenuButton::builder()
        .icon_name("view-more-symbolic")
        .menu_model(&menu)
        .valign(Align::Center)
        .css_classes(["flat"])
        .tooltip_text(tr("More").as_str())
        .build();

    heading.append(
        &Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build(),
    );
    heading.append(&overflow);
    heading.append(&switch);

    let empty = caption("");
    let list = boxed_list();

    let more_label = caption("");
    more_label.set_hexpand(true);
    more_label.set_valign(Align::Center);
    let more_button = Button::builder()
        .label(tr("Show more").as_str())
        .valign(Align::Center)
        .build();
    let more = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(8)
        .visible(false)
        .build();
    more.append(&more_label);
    more.append(&more_button);

    let widget = Box::builder().orientation(Orientation::Vertical).build();
    widget.append(&heading);
    widget.append(&empty);
    widget.append(&list);
    widget.append(&more);

    let clear = gtk::gio::SimpleAction::new("clear", None);
    let actions = gtk::gio::SimpleActionGroup::new();
    actions.add_action(&clear);
    widget.insert_action_group("journal", Some(&actions));

    let section = Rc::new(JournalSection {
        widget,
        list,
        empty,
        more,
        more_label,
        more_button,
        operations: RefCell::new(Vec::new()),
        shown: Cell::new(0),
        expanded: RefCell::new(std::collections::HashSet::new()),
        switch,
        overflow,
        _settings: settings,
        app_names: RefCell::new(HashMap::new()),
        on_undone: Rc::new(RefCell::new(None)),
        generation: Rc::new(Cell::new(0)),
        busy: Cell::new(false),
    });

    section.more_button.connect_clicked(clone!(
        #[weak]
        section,
        move |_| section.append_rows(MORE_ROWS)
    ));

    section.switch.connect_active_notify(clone!(
        #[weak]
        section,
        move |_| section.load()
    ));

    clear.connect_activate(clone!(
        #[weak]
        section,
        move |_, _| section.clone().clear_history()
    ));

    section
}

impl JournalSection {
    pub(super) fn connect_undone(&self, f: impl Fn(&[u32]) + 'static) {
        *self.on_undone.borrow_mut() = Some(Rc::new(f));
    }

    pub(super) fn set_app_names(&self, names: HashMap<u32, String>) {
        *self.app_names.borrow_mut() = names;
    }

    pub(super) fn load(self: &Rc<Self>) {
        MainContext::default().spawn_local(clone!(
            #[weak(rename_to = section)]
            self,
            async move { section.reload().await }
        ));
    }

    async fn reload(self: &Rc<Self>) {
        self.generation.set(self.generation.get().wrapping_add(1));
        let generation = self.generation.get();
        let keep = self.shown.get().max(FIRST_ROWS);
        let Ok(entries) = spawn_blocking(action_journal::load).await else {
            if self.generation.get() == generation {
                self.draw(Vec::new(), keep);
            }
            return;
        };
        if self.generation.get() != generation {
            return;
        }
        self.draw(action_journal::group(entries), keep);
    }

    fn scroll_adjustment(&self) -> Option<gtk::Adjustment> {
        self.widget
            .ancestor(gtk::ScrolledWindow::static_type())
            .and_downcast::<gtk::ScrolledWindow>()
            .map(|scroll| scroll.vadjustment())
    }

    /// Asking once cannot even tell whether it worked: the rows are up but not
    /// laid out, so the adjustment still describes the old extent and the next
    /// pass clamps the value. Hence asking again on every extent change until
    /// one holds, and giving up after a handful.
    fn restore_scroll(&self, offset: f64) {
        let Some(adjustment) = self.scroll_adjustment() else {
            return;
        };
        adjustment.set_value(offset);

        let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::new(RefCell::new(None));
        let attempts = Cell::new(0u8);
        let id = adjustment.connect_changed(clone!(
            #[strong]
            handler,
            move |adjustment| {
                adjustment.set_value(offset);
                attempts.set(attempts.get() + 1);
                if (adjustment.value() - offset).abs() >= 1.0 && attempts.get() < 8 {
                    return;
                }
                let id = handler.borrow_mut().take();
                if let Some(id) = id {
                    adjustment.disconnect(id);
                }
            }
        ));
        *handler.borrow_mut() = Some(id);
    }

    fn app_label(&self, app_id: u32, recorded: &str) -> String {
        if let Some(name) = self.app_names.borrow().get(&app_id)
            && !name.is_empty()
        {
            return name.clone();
        }
        if !recorded.is_empty() {
            return recorded.to_string();
        }
        tr("App {id}").replace("{id}", &app_id.to_string())
    }

    fn is_this_account(&self, operation: &Operation) -> bool {
        let current = action_journal::account();
        current == 0 || operation.account == 0 || operation.account == current
    }

    fn draw(self: &Rc<Self>, operations: Vec<Operation>, keep: usize) {
        clear_list(&self.list);
        self.overflow.set_visible(!operations.is_empty());

        if operations.is_empty() {
            self.list.set_visible(false);
            self.more.set_visible(false);
            self.empty.set_visible(true);
            self.empty.set_label(&if self.switch.is_active() {
                tr("Nothing recorded yet. From now on, every change made here lands in this list.")
            } else {
                tr(
                    "Off. Switch it on and SamRewritten will keep a list of the changes it makes, so you can undo them.",
                )
            });
            return;
        }

        self.empty.set_visible(false);
        self.list.set_visible(true);
        self.shown.set(0);
        *self.operations.borrow_mut() = operations;
        self.append_rows(keep);
    }

    fn append_rows(self: &Rc<Self>, count: usize) {
        let operations = self.operations.borrow();
        let from = self.shown.get();
        let to = (from + count).min(operations.len());
        for operation in &operations[from..to] {
            self.list.append(&self.build_row(operation));
        }
        self.shown.set(to);

        let hidden = operations.len() - to;
        self.more.set_visible(hidden > 0);
        if hidden > 0 {
            self.more_label.set_label(
                &tr("{count} older change(s) not shown").replace("{count}", &hidden.to_string()),
            );
        }
    }

    fn build_row(self: &Rc<Self>, operation: &Operation) -> ListBoxRow {
        let title = Label::builder()
            .label(self.title_for(operation))
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let text = Box::builder()
            .orientation(Orientation::Vertical)
            .valign(Align::Center)
            .hexpand(true)
            .margin_start(8)
            .build();
        text.append(&title);
        text.append(&caption(&self.subtitle_for(operation)));

        let header = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();
        header.append(&text);
        if let Some(button) = self.undo_button(operation, None) {
            header.append(&button);
        }

        // Built on first open, not now: twenty-five rows of an "unlock all"
        // came to a third of a second of frozen main loop.
        let expander = Expander::builder()
            .label_widget(&header)
            .hexpand(true)
            .build();
        let operation_for_expand = operation.clone();
        let batch = operation.batch;
        expander.connect_expanded_notify(clone!(
            #[weak(rename_to = section)]
            self,
            move |expander| {
                if expander.is_expanded() {
                    if expander.child().is_none() {
                        expander.set_child(Some(&section.changes_box(&operation_for_expand)));
                    }
                    section.expanded.borrow_mut().insert(batch);
                } else {
                    section.expanded.borrow_mut().remove(&batch);
                }
            }
        ));
        let was_open = self.expanded.borrow().contains(&batch);
        if was_open {
            expander.set_expanded(true);
        }

        let content = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();
        content.append(&expander);

        let row = ListBoxRow::builder()
            .child(&content)
            .activatable(false)
            .selectable(false)
            .build();
        if operation.reverted {
            row.add_css_class("dim-label");
        }
        row
    }

    fn changes_box(self: &Rc<Self>, operation: &Operation) -> Box {
        let list = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_start(12)
            .build();
        for (index, change) in operation.changes.iter().enumerate().take(MAX_CHANGES) {
            let line = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .build();
            let label = caption(&self.change_text(operation, change));
            label.set_hexpand(true);
            line.append(&label);
            if change.reverted {
                line.append(&caption(tr("Undone").as_str()));
            } else if let Some(button) = self.undo_button(operation, Some(index)) {
                button.add_css_class("flat");
                line.append(&button);
            }
            list.append(&line);
        }

        let hidden = operation.changes.len().saturating_sub(MAX_CHANGES);
        if hidden > 0 {
            list.append(&caption(
                &tr("and {count} more").replace("{count}", &hidden.to_string()),
            ));
        }
        list
    }

    fn undo_button(self: &Rc<Self>, operation: &Operation, index: Option<usize>) -> Option<Button> {
        let available = match index {
            None => operation.undoable(),
            Some(index) => operation
                .changes
                .get(index)
                .is_some_and(|c| c.undoable && !c.reverted),
        };
        if !available || !self.is_this_account(operation) {
            return None;
        }

        let button = Button::builder()
            .label(tr("Undo").as_str())
            .valign(Align::Center)
            .build();
        let operation = operation.clone();
        button.connect_clicked(clone!(
            #[weak(rename_to = section)]
            self,
            move |_| section.clone().undo(operation.clone(), index)
        ));
        Some(button)
    }

    fn title_for(&self, operation: &Operation) -> String {
        let where_ = if operation.apps.len() > 1 {
            tr("{count} games").replace("{count}", &operation.apps.len().to_string())
        } else {
            self.app_label(operation.app_id, &operation.app_name)
        };

        let achievements: Vec<&RecordedChange> = operation
            .changes
            .iter()
            .filter(|c| matches!(c.change, Change::Achievement { .. }))
            .collect();
        if !achievements.is_empty() {
            let unlocking = achievements
                .iter()
                .all(|c| matches!(c.change, Change::Achievement { after: true, .. }));
            let locking = achievements
                .iter()
                .all(|c| matches!(c.change, Change::Achievement { after: false, .. }));
            let template = if unlocking {
                tr("Unlocked {count} achievement(s) in {app}")
            } else if locking {
                tr("Locked {count} achievement(s) in {app}")
            } else {
                tr("Changed {count} achievement(s) in {app}")
            };
            return template
                .replace("{count}", &achievements.len().to_string())
                .replace("{app}", &where_);
        }

        let template = match operation.op {
            Op::StatEdit => tr("Changed {count} stat(s) in {app}"),
            Op::ResetApp | Op::BulkLock => tr("Reset {app}"),
            Op::BulkUnlock => tr("Unlocked everything in {app}"),
            Op::Import => tr("Wrote a file's progress into {app}"),
            _ => tr("Changed {count} thing(s) in {app}"),
        };
        template
            .replace("{count}", &operation.changes.len().to_string())
            .replace("{app}", &where_)
    }

    fn subtitle_for(&self, operation: &Operation) -> String {
        let mut parts = Vec::new();
        if let Some(day) = format_stamp(operation.at) {
            parts.push(day);
        }
        if operation.op == Op::Revert {
            parts.push(tr("an undo").to_string());
        }
        if operation.reverted {
            parts.push(tr("Undone").to_string());
        } else if operation.changes.iter().any(|c| c.reverted) {
            parts.push(tr("partly undone").to_string());
        } else if !operation.changes.iter().any(|c| c.undoable) {
            parts.push(tr("cannot be undone from here").to_string());
        } else if !self.is_this_account(operation) {
            parts.push(tr("another Steam account").to_string());
        }
        parts.join(" · ")
    }

    fn change_text(&self, operation: &Operation, recorded: &RecordedChange) -> String {
        let label = recorded.change.label();
        match &recorded.change {
            Change::Achievement { after: true, .. } => {
                tr("{name} — unlocked").replace("{name}", label)
            }
            Change::Achievement { after: false, .. } => {
                tr("{name} — locked").replace("{name}", label)
            }
            Change::IntStat { before, .. } if operation.op == Op::ResetApp => {
                tr("{name} — was {value}")
                    .replace("{name}", label)
                    .replace("{value}", &before.to_string())
            }
            Change::FloatStat { before, .. } if operation.op == Op::ResetApp => {
                tr("{name} — was {value}")
                    .replace("{name}", label)
                    .replace("{value}", &before.to_string())
            }
            Change::IntStat { before, after, .. } => tr("{name} — {before} to {after}")
                .replace("{name}", label)
                .replace("{before}", &before.to_string())
                .replace("{after}", &after.to_string()),
            Change::FloatStat { before, after, .. } => tr("{name} — {before} to {after}")
                .replace("{name}", label)
                .replace("{before}", &before.to_string())
                .replace("{after}", &after.to_string()),
            Change::Opaque { detail } => {
                let app = self.app_label(recorded.app_id, &recorded.app_name);
                format!("{app} — {}", tr(detail))
            }
        }
    }

    fn clear_history(self: Rc<Self>) {
        MainContext::default().spawn_local(async move {
            let window = self
                .widget
                .root()
                .and_then(|root| root.downcast::<gtk::Window>().ok());
            let go_ahead = confirm_dialog(
                window.as_ref(),
                tr("Delete this history?").as_str(),
                tr("The list is emptied for good. Nothing is put back. Whatever was changed in your games stays changed.").as_str(),
                tr("Delete").as_str(),
                true,
            )
            .await;
            if !go_ahead {
                return;
            }
            let _ = spawn_blocking(action_journal::clear).await;
            self.load();
        });
    }

    fn undo(self: Rc<Self>, operation: Operation, index: Option<usize>) {
        if self.busy.replace(true) {
            return;
        }
        MainContext::default().spawn_local(async move {
            // Held over the redraw too: released any earlier, a second click
            // would land on a stale row and re-apply values already back.
            self.clone().run_undo(operation, index).await;
            self.busy.set(false);
        });
    }

    async fn run_undo(self: Rc<Self>, operation: Operation, index: Option<usize>) {
        let picked: Vec<(usize, &RecordedChange)> = match index {
            Some(index) => operation
                .changes
                .iter()
                .enumerate()
                .filter(|(i, c)| *i == index && c.undoable && !c.reverted)
                .collect(),
            None => operation
                .changes
                .iter()
                .enumerate()
                .filter(|(_, c)| c.undoable && !c.reverted)
                .collect(),
        };
        if picked.is_empty() {
            return;
        }

        let window = self
            .widget
            .root()
            .and_then(|root| root.downcast::<gtk::Window>().ok());
        let app_name = self.app_label(operation.app_id, &operation.app_name);

        let went_ahead = confirm_dialog(
            window.as_ref(),
            tr("Undo this change?").as_str(),
            &tr("{count} change(s) in {app} will be put back the way they were.")
                .replace("{count}", &picked.len().to_string())
                .replace("{app}", &app_name),
            tr("Undo").as_str(),
            false,
        )
        .await;
        if !went_ahead {
            return;
        }

        let mut export = AppExport {
            app_id: operation.app_id,
            app_name: app_name.clone(),
            achievements: Vec::new(),
            stats: Vec::new(),
        };
        let mut inverse = Vec::new();
        for (_, recorded) in &picked {
            match &recorded.change {
                Change::Achievement {
                    id,
                    name,
                    before,
                    after,
                } => {
                    export.achievements.push(AppAchievementExport {
                        id: id.clone(),
                        is_achieved: *before,
                        permission: 0,
                    });
                    inverse.push(Change::Achievement {
                        id: id.clone(),
                        name: name.clone(),
                        before: *after,
                        after: *before,
                    });
                }
                Change::IntStat {
                    id,
                    name,
                    before,
                    after,
                } => {
                    export.stats.push(AppStatExport {
                        id: id.clone(),
                        value: AppStatValue::Int(*before),
                        permission: 0,
                    });
                    inverse.push(Change::IntStat {
                        id: id.clone(),
                        name: name.clone(),
                        before: *after,
                        after: *before,
                    });
                }
                Change::FloatStat {
                    id,
                    name,
                    before,
                    after,
                } => {
                    export.stats.push(AppStatExport {
                        id: id.clone(),
                        value: AppStatValue::Float(*before),
                        permission: 0,
                    });
                    inverse.push(Change::FloatStat {
                        id: id.clone(),
                        name: name.clone(),
                        before: *after,
                        after: *before,
                    });
                }
                Change::Opaque { .. } => {}
            }
        }

        let app_id = operation.app_id;
        let Ok(result) = spawn_blocking(move || {
            (ImportApps { apps: vec![export] }).request_with_progress(|_, _| {})
        })
        .await
        else {
            return;
        };

        let mut problems: Vec<String> = Vec::new();
        let mut applied = 0usize;
        // A refusal Steam names is one this can work around: everything it did
        // not name came back. A failure with no summary names nothing, so none
        // of it may be filed as put back.
        let mut unaccounted = false;
        match result {
            Ok(results) => {
                if results.is_empty() {
                    unaccounted = true;
                }
                for (_, res) in results {
                    match res {
                        Ok(summary) => {
                            applied += summary.achievements_applied + summary.stats_applied;
                            // Steam holds every set until the store commits,
                            // so a failed store took the whole app with it.
                            if summary.errors.iter().any(|e| e.starts_with("store failed")) {
                                unaccounted = true;
                            }
                            problems.extend(summary.errors);
                            problems.extend(summary.skipped_protected);
                            problems.extend(summary.skipped_unwriteable);
                        }
                        Err(e) => {
                            problems.push(e.to_string());
                            unaccounted = true;
                        }
                    }
                }
            }
            Err(e) => {
                problems.push(e.to_string());
                unaccounted = true;
            }
        }

        // Lifted out of the cell first: a handler reaching back into
        // `connect_undone` would panic on the borrow.
        let on_undone = self.on_undone.borrow().clone();
        if applied > 0
            && let Some(on_undone) = on_undone
        {
            on_undone(&[app_id]);
        }

        // Filed change by change: recording a half-refused batch whole claims
        // values still at zero were restored, and recording none of it leaves a
        // row whose every change is back still asking to be undone.
        let put_back: Vec<Change> = if unaccounted {
            Vec::new()
        } else {
            inverse
                .into_iter()
                .filter(|change| !refused(&problems, change))
                .collect()
        };
        let put_back_count = put_back.len();
        if !put_back.is_empty() {
            Batch::reversing(
                app_id,
                app_name,
                Reverses {
                    batch: operation.batch,
                },
            )
            .record(put_back);
        }

        // The clicked button goes away with its row. Left holding the focus,
        // GTK moves it to the top of the rebuilt list and scrolls there,
        // undoing the restore below.
        if let Some(window) = &window {
            gtk::prelude::GtkWindowExt::set_focus(window, None::<&gtk::Widget>);
        }

        if !problems.is_empty()
            && let Some(window) = window
        {
            let body = if put_back_count > 0 {
                tr("{count} change(s) were put back. These could not:")
                    .replace("{count}", &put_back_count.to_string())
            } else {
                tr("An unexpected failure occurred while putting this back:").to_string()
            };
            show_list_dialog(
                &window,
                tr("Undo incomplete").as_str(),
                &body,
                &problems.join("\n"),
            );
        }

        let offset = self
            .scroll_adjustment()
            .map(|adjustment| adjustment.value());
        clear_list(&self.list);
        self.more.set_visible(false);
        self.reload().await;
        if let Some(offset) = offset {
            self.restore_scroll(offset);
        }
    }
}

/// The only thing tying a refusal back to a change is the `ach:ID` / `stat:ID`
/// head `progress_io` puts on every line it reports.
fn refused(problems: &[String], change: &Change) -> bool {
    let head = match change {
        Change::Achievement { id, .. } => format!("ach:{id}"),
        Change::IntStat { id, .. } | Change::FloatStat { id, .. } => format!("stat:{id}"),
        Change::Opaque { .. } => return true,
    };
    let with_reason = format!("{head} ");
    problems
        .iter()
        .any(|p| *p == head || p.starts_with(&with_reason))
}

fn format_stamp(unix_seconds: u64) -> Option<String> {
    gtk::glib::DateTime::from_unix_local(unix_seconds as i64)
        .ok()?
        .format("%x %H:%M")
        .ok()
        .map(|s| s.to_string())
}
