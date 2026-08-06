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

//! The page behind the sidebar's profile card.
//!
//! Two sources, and they never mix. The library figures come from the app list
//! store the grid is already showing, so they are exactly as complete as that
//! store is — which the page says out loud rather than averaging over half a
//! library. The activity grid is parsed from Steam's own on-disk files.

mod completion_graph;
mod heatmap;
pub(crate) mod identity;
mod journal_section;
mod timeline;

use crate::backend::user_unlock_times::{account_id, read_all_unlock_stamps};
use crate::gui_frontend::gobjects::steam_app::GSteamAppObject;
use crate::gui_frontend::i18n::{tr, tr_noop};
use crate::gui_frontend::widgets::clamp::Clamp;
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use completion_graph::CompletionGraph;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::{MainContext, clone};
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, DropDown, FlowBox, Frame, Image, Label, ListBox, ListBoxRow, Orientation,
    PolicyType, ProgressBar, ScrolledWindow, SelectionMode, Spinner, Stack, StringList,
};
use heatmap::{DaySummary, Heatmap};
use identity::SharedIdentity;
use journal_section::{JournalSection, build_journal_section};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

const AVATAR_SIZE: i32 = 96;

const MAX_CONTENT_WIDTH: i32 = 980;

const BURSTS_MAX_ROWS: usize = 25;

const LIBRARY_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

const HISTORY_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// `needs_counts` marks the tiles summed over per-app counts, which arrive
/// visible-cards-first. Half a library does not average out to a smaller truth,
/// so those tiles show nothing until every game has been read.
struct TileSpec {
    label: &'static str,
    needs_counts: bool,
}

const TILES: &[TileSpec] = &[
    TileSpec {
        label: tr_noop("Games in library"),
        needs_counts: false,
    },
    TileSpec {
        label: tr_noop("Achievements unlocked"),
        needs_counts: true,
    },
    TileSpec {
        label: tr_noop("Average completion"),
        needs_counts: true,
    },
    TileSpec {
        label: tr_noop("Perfect games"),
        needs_counts: true,
    },
    TileSpec {
        label: tr_noop("Time played"),
        needs_counts: false,
    },
];

struct Tile {
    value: Label,
    faces: Stack,
    needs_counts: bool,
}

type SelectApps = Rc<RefCell<Option<std::boxed::Box<dyn Fn(&[u32])>>>>;

/// Playtime below which a run has nothing behind it. Counted per account across
/// machines, so a second PC does not fool this; a console save does.
const UNPLAYED_MINUTES: u32 = 30;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reading {
    SeveralGames,
    NeverPlayed,
    WholeGame,
    MidHistory,
    CouldBeBackfill,
}

fn reads_as(burst: &timeline::Burst, app: Option<&GSteamAppObject>) -> Reading {
    if burst.cluster_apps > 1 {
        return Reading::SeveralGames;
    }
    if app.is_some_and(|app| app.playtime_minutes() < UNPLAYED_MINUTES) {
        return Reading::NeverPlayed;
    }
    match (burst.earlier, burst.later, burst.ordinary_later) {
        (0, 0, _) => Reading::WholeGame,
        (0, _, 1..) => Reading::CouldBeBackfill,
        _ => Reading::MidHistory,
    }
}

enum Row<'a> {
    Sitting {
        start: u32,
        games: usize,
        count: usize,
        apps: Vec<u32>,
    },
    Single(Reading, &'a timeline::Burst),
}

impl Row<'_> {
    fn rank(&self) -> (Reading, std::cmp::Reverse<u32>) {
        match self {
            Row::Sitting { start, .. } => (Reading::SeveralGames, std::cmp::Reverse(*start)),
            Row::Single(reading, burst) => (*reading, std::cmp::Reverse(burst.start)),
        }
    }
}

fn rows_for<'a>(
    bursts: &'a [timeline::Burst],
    apps: &HashMap<u32, GSteamAppObject>,
) -> Vec<Row<'a>> {
    let mut sittings: HashMap<usize, Row> = HashMap::new();
    let mut rows = Vec::new();
    for burst in bursts {
        if burst.cluster_apps < 2 {
            rows.push(Row::Single(reads_as(burst, apps.get(&burst.app_id)), burst));
            continue;
        }
        match sittings.entry(burst.cluster).or_insert(Row::Sitting {
            start: burst.start,
            games: burst.cluster_apps,
            count: 0,
            apps: Vec::new(),
        }) {
            Row::Sitting {
                start, count, apps, ..
            } => {
                *start = (*start).min(burst.start);
                *count += burst.count;
                if !apps.contains(&burst.app_id) {
                    apps.push(burst.app_id);
                }
            }
            Row::Single(..) => unreachable!("a sitting is never stored as a single run"),
        }
    }
    rows.extend(sittings.into_values());
    rows.sort_by_key(Row::rank);
    rows
}

fn reading_text(reading: Reading, app: Option<&GSteamAppObject>) -> String {
    match reading {
        Reading::SeveralGames => unreachable!("a sitting never reaches a single run's reading"),
        Reading::NeverPlayed => match app.map(|app| app.playtime_minutes()) {
            Some(0) | None => tr("never played").to_string(),
            Some(minutes) => {
                tr("{minutes} minute(s) played, ever").replace("{minutes}", &minutes.to_string())
            }
        },
        Reading::WholeGame => {
            tr("nothing else was unlocked for this game outside this moment").to_string()
        }
        Reading::MidHistory => {
            tr("the rest of your unlocks in this game came at other times").to_string()
        }
        Reading::CouldBeBackfill => {
            tr("at the very start of this game's history, ordinary unlocks after it").to_string()
        }
    }
}

#[derive(Default)]
struct LibraryStats {
    apps: u32,
    measured: u32,
    unlocked: u64,
    total: u64,
    perfect: u32,
    playtime_minutes: u64,
    /// A library is mostly games never opened, and averaging those in measures
    /// the backlog rather than the playing; Steam leaves them out too.
    started: u32,
    started_rate_sum: f64,
}

fn collect_stats(list_store: &ListStore) -> LibraryStats {
    let mut stats = LibraryStats::default();
    for i in 0..list_store.n_items() {
        let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>() else {
            continue;
        };
        if app.is_synthetic() || app.is_junk() {
            continue;
        }
        stats.apps += 1;
        stats.playtime_minutes += u64::from(app.playtime_minutes());
        if !app.achievements_loaded() {
            continue;
        }
        stats.measured += 1;
        let total = app.achievement_count();
        let unlocked = app.unlocked_achievement_count();
        stats.total += u64::from(total);
        stats.unlocked += u64::from(unlocked);
        if total > 0 && unlocked >= total {
            stats.perfect += 1;
        }
        if total > 0 && unlocked > 0 {
            stats.started += 1;
            stats.started_rate_sum += f64::from(unlocked.min(total)) / f64::from(total);
        }
    }
    stats
}

fn completion_percent(stats: &LibraryStats) -> u64 {
    if stats.started == 0 {
        return 0;
    }
    (stats.started_rate_sum / f64::from(stats.started) * 100.0).round() as u64
}

/// Kept so switching year is a redraw, not another sweep of Steam's files.
#[derive(Default)]
struct HeatmapData {
    per_day: HashMap<i32, DaySummary>,
    total: usize,
    since: Option<String>,
    /// Newest first, mirroring the year dropdown's rows.
    years: Vec<i32>,
}

/// The current year ends today rather than on 31 December, so the grid never
/// trails empty weeks.
fn draw_heatmap(heatmap: &Heatmap, caption: &Label, data: &HeatmapData, selected: u32) {
    let Some(&year) = data.years.get(selected as usize) else {
        caption.set_label(&tr("No unlocks yet."));
        return;
    };
    let today = timeline::today();
    let end_day = if year == timeline::civil_from_days(today).0 {
        today
    } else {
        timeline::days_from_civil(year, 12, 31)
    };
    heatmap.set_data(&data.per_day, end_day);

    // The grid reaches back into the previous December; the count should not.
    let from = if end_day == today {
        heatmap::grid_start(end_day)
    } else {
        timeline::days_from_civil(year, 1, 1)
    };
    let drawn: u32 = data
        .per_day
        .iter()
        .filter(|&(&day, _)| (from..=end_day).contains(&day))
        .map(|(_, summary)| summary.count)
        .sum();
    let Some(since) = data.since.as_ref() else {
        caption.set_label(&tr("No unlocks yet."));
        return;
    };
    caption.set_label(&if drawn as usize == data.total {
        tr("{total} unlocked since {since}.")
            .replace("{total}", &data.total.to_string())
            .replace("{since}", since)
    } else if end_day == today {
        tr("{year} in the last year, {total} since {since}.")
            .replace("{year}", &drawn.to_string())
            .replace("{total}", &data.total.to_string())
            .replace("{since}", since)
    } else {
        tr("{count} in {year}, {total} since {since}.")
            .replace("{count}", &drawn.to_string())
            .replace("{year}", &year.to_string())
            .replace("{total}", &data.total.to_string())
            .replace("{since}", since)
    });
}

/// Only ever a file deleted since Steam last started: asking for a game's stats
/// is what makes Steam write it. Zero until every game has been read.
fn missing_from_cache(apps: &HashMap<u32, GSteamAppObject>, cached: &HashSet<u32>) -> u32 {
    let mut missing = 0;
    for app in apps.values() {
        if app.is_synthetic() || app.is_junk() {
            continue;
        }
        if !app.achievements_loaded() {
            return 0;
        }
        if app.achievement_count() > 0 && !cached.contains(&app.app_id()) {
            missing += 1;
        }
    }
    missing
}

fn apps_by_id(list_store: &ListStore) -> HashMap<u32, GSteamAppObject> {
    let mut apps = HashMap::new();
    for i in 0..list_store.n_items() {
        if let Some(app) = list_store.item(i).and_downcast::<GSteamAppObject>() {
            apps.insert(app.app_id(), app);
        }
    }
    apps
}

fn section_heading(text: &str) -> (Box, Spinner) {
    let spinner = Spinner::builder()
        .valign(Align::Center)
        .visible(false)
        .spinning(true)
        .build();
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_top(18)
        .build();
    row.append(
        &Label::builder()
            .label(text)
            .xalign(0.0)
            .css_classes(["title-4"])
            .build(),
    );
    row.append(&spinner);
    (row, spinner)
}

fn caption(text: &str) -> Label {
    Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build()
}

fn list_row(title: &str, subtitle: &str, action: Option<&Button>) -> ListBoxRow {
    let title_label = Label::builder()
        .label(title)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    let text = Box::builder()
        .orientation(Orientation::Vertical)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    text.append(&title_label);
    text.append(&caption(subtitle));

    let content = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&text);
    if let Some(action) = action {
        content.append(action);
    }

    ListBoxRow::builder()
        .child(&content)
        .activatable(false)
        .selectable(false)
        .build()
}

fn boxed_list() -> ListBox {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_top(6)
        .build();
    list.set_visible(false);
    list
}

fn clear_list(list: &ListBox) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }
}

pub(crate) struct ProfileView {
    pub widget: ScrolledWindow,
    identity: SharedIdentity,
    counts_loading: Rc<dyn Fn() -> bool>,
    avatar: ShimmerImage,
    name_label: Label,
    steam_id_label: Label,
    tiles: Vec<Tile>,
    coverage_row: Box,
    coverage_label: Label,
    coverage_progress: ProgressBar,
    measure_button: Button,
    history_spinner: Spinner,
    bursts_spinner: Spinner,
    heatmap: Heatmap,
    heatmap_caption: Label,
    heatmap_years: DropDown,
    heatmap_years_handler: Rc<gtk::glib::SignalHandlerId>,
    heatmap_year_model: StringList,
    heatmap_data: Rc<RefCell<HeatmapData>>,
    completion_spinner: Spinner,
    completion_graph: CompletionGraph,
    completion_caption: Label,
    bursts_list: ListBox,
    bursts_clean: Label,
    bursts_more: Label,
    cache_banner: Frame,
    cache_banner_label: Label,
    journal: Rc<JournalSection>,
    on_open_app: Rc<dyn Fn(&GSteamAppObject)>,
    select_apps: SelectApps,
    generation: Rc<Cell<u64>>,
    history_drawn: Rc<Cell<bool>>,
    library_refresh_queued: Cell<bool>,
    history_refresh_queued: Cell<bool>,
}

pub(crate) fn build_profile_view(
    identity: SharedIdentity,
    on_open_app: Rc<dyn Fn(&GSteamAppObject)>,
    on_measure_all: Rc<dyn Fn()>,
    counts_loading: Rc<dyn Fn() -> bool>,
) -> ProfileView {
    let avatar = ShimmerImage::new();
    avatar.set_size_request(AVATAR_SIZE, AVATAR_SIZE);
    avatar.set_valign(Align::Center);

    let name_label = Label::builder()
        .label(tr("Steam user").as_str())
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["title-1"])
        .build();
    let steam_id_label = Label::builder()
        .label("—")
        .xalign(0.0)
        .selectable(true)
        .css_classes(["dim-label", "numeric"])
        .build();
    let header_text = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .valign(Align::Center)
        .hexpand(true)
        .build();
    header_text.append(&name_label);
    header_text.append(&steam_id_label);
    let header = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(18)
        .build();
    header.append(&avatar);
    header.append(&header_text);

    let tiles = FlowBox::builder()
        .selection_mode(SelectionMode::None)
        .homogeneous(true)
        .min_children_per_line(2)
        .max_children_per_line(TILES.len() as u32)
        // Zero, not 6: the stock stylesheet already pads every flowboxchild by 3px.
        .row_spacing(0)
        .column_spacing(0)
        .margin_top(18)
        .build();
    let mut tile_widgets = Vec::with_capacity(TILES.len());
    for spec in TILES {
        let value = Label::builder()
            .xalign(0.0)
            .css_classes(["title-2", "numeric"])
            .build();
        let unknown = Label::builder()
            .label("—")
            .xalign(0.0)
            .css_classes(["title-2", "numeric", "dim-label"])
            .build();
        let spinner = Spinner::builder()
            .halign(Align::Start)
            .valign(Align::Center)
            .spinning(true)
            .build();
        let faces = Stack::builder().build();
        faces.add_named(&value, Some("value"));
        faces.add_named(&unknown, Some("unknown"));
        faces.add_named(&spinner, Some("loading"));
        faces.set_visible_child_name("unknown");

        let tile = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        tile.append(&faces);
        tile.append(&caption(tr(spec.label).as_str()));
        tiles.append(&Frame::builder().child(&tile).build());
        tile_widgets.push(Tile {
            value,
            faces,
            needs_counts: spec.needs_counts,
        });
    }

    let coverage_label = caption("");
    coverage_label.set_hexpand(true);
    coverage_label.set_valign(Align::Center);
    let measure_button = Button::builder()
        .label(tr("Read all my games").as_str())
        .valign(Align::Center)
        .build();
    measure_button.connect_clicked(move |_| on_measure_all());
    let coverage_text = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    coverage_text.append(&coverage_label);
    coverage_text.append(&measure_button);
    let coverage_progress = ProgressBar::builder().margin_top(6).visible(false).build();
    let coverage_row = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(10)
        .visible(false)
        .build();
    coverage_row.append(&coverage_text);
    coverage_row.append(&coverage_progress);

    let heatmap = Heatmap::default();
    heatmap.set_halign(Align::Center);
    heatmap.set_margin_top(10);
    heatmap.set_margin_bottom(10);
    heatmap.set_margin_start(10);
    heatmap.set_margin_end(10);
    let heatmap_frame = Frame::builder().child(&heatmap).margin_top(6).build();

    let heatmap_caption = caption("");
    let heatmap_year_model = StringList::new(&[]);
    let heatmap_years = DropDown::builder()
        .model(&heatmap_year_model)
        .visible(false)
        .valign(Align::Center)
        .css_classes(["flat"])
        .tooltip_text(tr("Year to show").as_str())
        .build();

    let completion_graph = CompletionGraph::default();
    completion_graph.set_margin_top(10);
    completion_graph.set_margin_bottom(10);
    completion_graph.set_margin_start(10);
    completion_graph.set_margin_end(10);
    let completion_frame = Frame::builder()
        .child(&completion_graph)
        .margin_top(6)
        .build();
    let completion_caption = caption("");

    let bursts_list = boxed_list();
    let bursts_clean = caption(tr("Everything here looks like ordinary play.").as_str());
    let bursts_more = caption("");
    bursts_more.set_visible(false);

    // `.warning` only on the icon: Greybird paints the class as a filled
    // background, which over a paragraph reads as selected text.
    let cache_banner_label = Label::builder()
        .xalign(0.0)
        .wrap(true)
        .hexpand(true)
        .build();
    let cache_banner_row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(12)
        .margin_end(12)
        .build();
    let cache_banner_icon = Image::from_icon_name("dialog-warning-symbolic");
    cache_banner_icon.set_valign(Align::Start);
    cache_banner_icon.add_css_class("warning");
    cache_banner_row.append(&cache_banner_icon);
    cache_banner_row.append(&cache_banner_label);
    let cache_banner = Frame::builder()
        .child(&cache_banner_row)
        .margin_bottom(12)
        .visible(false)
        .build();

    let content = Box::builder()
        .orientation(Orientation::Vertical)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&cache_banner);
    content.append(&header);
    content.append(&tiles);
    content.append(&coverage_row);

    // Its own column, not a row with the text: a dropdown is taller than a line
    // of text and would stretch the spacing around whichever line it sat beside.
    let (history_heading, history_spinner) = section_heading(tr("Unlock activity").as_str());
    let heatmap_text = Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .build();
    heatmap_text.append(&history_heading);
    heatmap_text.append(&heatmap_caption);
    let heatmap_header = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    heatmap_header.append(&heatmap_text);
    heatmap_header.append(&heatmap_years);

    content.append(&heatmap_header);
    content.append(&heatmap_frame);

    let (completion_heading, completion_spinner) =
        section_heading(tr("Completion over time").as_str());
    content.append(&completion_heading);
    content.append(&completion_caption);
    content.append(&completion_frame);

    let (bursts_heading, bursts_spinner) =
        section_heading(tr("Irregular activity history").as_str());
    content.append(&bursts_heading);
    content.append(&caption(
        tr("Anyone visiting your Steam profile sees when each achievement was unlocked. Listed here are the moments that do not look like ordinary play.").as_str(),
    ));
    content.append(&bursts_clean);
    content.append(&bursts_list);
    content.append(&bursts_more);

    let journal = build_journal_section();
    content.append(&journal.widget);

    let widget = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vexpand(true)
        .child(&Clamp::new(&content, MAX_CONTENT_WIDTH))
        .build();

    let heatmap_data: Rc<RefCell<HeatmapData>> = Rc::default();
    let heatmap_years_handler = Rc::new(heatmap_years.connect_selected_notify(clone!(
        #[strong]
        heatmap,
        #[strong]
        heatmap_caption,
        #[strong]
        heatmap_data,
        move |years| {
            let data = heatmap_data.borrow();
            if !data.years.is_empty() {
                draw_heatmap(&heatmap, &heatmap_caption, &data, years.selected());
            }
        }
    )));

    ProfileView {
        widget,
        identity,
        counts_loading,
        avatar,
        name_label,
        steam_id_label,
        tiles: tile_widgets,
        coverage_row,
        coverage_label,
        coverage_progress,
        measure_button,
        history_spinner,
        bursts_spinner,
        heatmap,
        heatmap_caption,
        heatmap_years,
        heatmap_years_handler,
        heatmap_year_model,
        heatmap_data,
        completion_spinner,
        completion_graph,
        completion_caption,
        bursts_list,
        bursts_clean,
        bursts_more,
        cache_banner,
        cache_banner_label,
        journal,
        on_open_app,
        select_apps: SelectApps::default(),
        generation: Rc::new(Cell::new(0)),
        history_drawn: Rc::new(Cell::new(false)),
        library_refresh_queued: Cell::new(false),
        history_refresh_queued: Cell::new(false),
    }
}

impl ProfileView {
    pub(crate) fn connect_select_apps(&self, f: impl Fn(&[u32]) + 'static) {
        *self.select_apps.borrow_mut() = Some(std::boxed::Box::new(f));
    }

    pub(crate) fn refresh_identity(&self) {
        let steam_id64 = self.identity.steam_id64.get();
        self.steam_id_label.set_label(&if steam_id64 == 0 {
            "—".to_string()
        } else {
            steam_id64.to_string()
        });

        let persona = self.identity.persona.borrow();
        if !persona.is_empty() {
            self.name_label.set_label(&persona);
        }
        if let Some(image) = self.identity.avatar.borrow().as_ref() {
            self.avatar
                .set_rgba(image.width as i32, image.height as i32, &image.rgba);
        }
    }

    pub(crate) fn load(&self, list_store: &ListStore) {
        self.refresh_library(list_store);
        self.load_timeline(list_store);
        self.journal.set_app_names(
            apps_by_id(list_store)
                .into_iter()
                .map(|(id, app)| (id, app.app_name()))
                .collect(),
        );
        self.journal.load();
    }

    pub(crate) fn connect_undone(&self, f: impl Fn(&[u32]) + 'static) {
        self.journal.connect_undone(f);
    }

    pub(crate) fn queue_refresh(self: &Rc<Self>, list_store: &ListStore) {
        if !self.library_refresh_queued.replace(true) {
            let view = self.clone();
            let list_store = list_store.clone();
            gtk::glib::timeout_add_local_once(LIBRARY_REFRESH_INTERVAL, move || {
                view.library_refresh_queued.set(false);
                view.refresh_library(&list_store);
            });
        }
        if !self.history_refresh_queued.replace(true) {
            let view = self.clone();
            let list_store = list_store.clone();
            gtk::glib::timeout_add_local_once(HISTORY_REFRESH_INTERVAL, move || {
                view.history_refresh_queued.set(false);
                view.load_timeline(&list_store);
            });
        }
    }

    pub(crate) fn refresh_library(&self, list_store: &ListStore) {
        let stats = collect_stats(list_store);
        let percent = completion_percent(&stats);
        let values = [
            stats.apps.to_string(),
            format!("{} / {}", stats.unlocked, stats.total),
            format!("{percent}%"),
            stats.perfect.to_string(),
            tr("{hours} h").replace("{hours}", &(stats.playtime_minutes / 60).to_string()),
        ];
        let complete = stats.measured >= stats.apps;
        let loading = (self.counts_loading)();
        for (tile, value) in self.tiles.iter().zip(values) {
            tile.value.set_label(&value);
            tile.faces
                .set_visible_child_name(match tile.needs_counts && !complete {
                    false => "value",
                    true if loading => "loading",
                    true => "unknown",
                });
        }

        self.coverage_row.set_visible(!complete);
        self.coverage_progress.set_visible(loading);
        self.measure_button.set_visible(!loading);
        self.history_spinner.set_visible(loading);
        self.completion_spinner.set_visible(loading);
        self.bursts_spinner.set_visible(loading);
        if !complete {
            let text = if loading {
                self.coverage_progress.set_fraction(if stats.apps > 0 {
                    f64::from(stats.measured) / f64::from(stats.apps)
                } else {
                    0.0
                });
                tr(
                    "Reading your games' achievements ({measured}/{total}). The activity and the runs below fill in as it goes.",
                )
            } else {
                tr(
                    "Only {measured} of your {total} games have been read from Steam, so the figures above cannot be totalled yet.",
                )
            };
            self.coverage_label.set_label(
                &text
                    .replace("{measured}", &stats.measured.to_string())
                    .replace("{total}", &stats.apps.to_string()),
            );
        }
    }

    fn load_timeline(&self, list_store: &ListStore) {
        let steam_id64 = self.identity.steam_id64.get();
        if steam_id64 == 0 {
            self.heatmap_caption
                .set_label(&tr("Waiting for Steam to say who is signed in."));
            return;
        }

        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        if !self.history_drawn.get() {
            self.heatmap_caption
                .set_label(&tr("Reading your unlock history…"));
        }

        let account = account_id(steam_id64);
        let handle = spawn_blocking(move || read_all_unlock_stamps(account).map(timeline::build));
        let apps = apps_by_id(list_store);
        MainContext::default().spawn_local(clone!(
            #[strong(rename_to = heatmap)]
            self.heatmap,
            #[strong(rename_to = heatmap_caption)]
            self.heatmap_caption,
            #[strong(rename_to = heatmap_years)]
            self.heatmap_years,
            #[strong(rename_to = heatmap_years_handler)]
            self.heatmap_years_handler,
            #[strong(rename_to = heatmap_year_model)]
            self.heatmap_year_model,
            #[strong(rename_to = heatmap_data)]
            self.heatmap_data,
            #[strong(rename_to = completion_graph)]
            self.completion_graph,
            #[strong(rename_to = completion_caption)]
            self.completion_caption,
            #[strong(rename_to = bursts_list)]
            self.bursts_list,
            #[strong(rename_to = bursts_clean)]
            self.bursts_clean,
            #[strong(rename_to = bursts_more)]
            self.bursts_more,
            #[strong(rename_to = cache_banner)]
            self.cache_banner,
            #[strong(rename_to = cache_banner_label)]
            self.cache_banner_label,
            #[strong(rename_to = on_open_app)]
            self.on_open_app,
            #[strong(rename_to = select_apps)]
            self.select_apps,
            #[strong(rename_to = current_generation)]
            self.generation,
            #[strong(rename_to = history_drawn)]
            self.history_drawn,
            async move {
                let timeline = match handle.await {
                    Ok(Ok(timeline)) => timeline,
                    Ok(Err(e)) => {
                        eprintln!("[CLIENT] Could not read the local unlock cache: {e}");
                        heatmap_caption.set_label(&tr("Your unlock history could not be read."));
                        return;
                    }
                    Err(e) => {
                        eprintln!("[CLIENT] Unlock cache sweep panicked: {e:?}");
                        heatmap_caption.set_label(&tr("Your unlock history could not be read."));
                        return;
                    }
                };
                if current_generation.get() != generation {
                    return;
                }

                history_drawn.set(true);
                let today = timeline::today();

                let this_year = timeline::civil_from_days(today).0;
                let earliest = timeline
                    .per_day
                    .keys()
                    .min()
                    .map(|day| timeline::civil_from_days(*day).0)
                    .unwrap_or(this_year);
                let years: Vec<i32> = (earliest..=this_year).rev().collect();
                let labels: Vec<String> = years.iter().map(|year| year.to_string()).collect();

                // A reload should leave you looking at the year you were on.
                let was = heatmap_data
                    .borrow()
                    .years
                    .get(heatmap_years.selected() as usize)
                    .copied();
                let per_day: HashMap<i32, DaySummary> = timeline
                    .per_day
                    .iter()
                    .map(|(day, count)| {
                        let games = timeline.apps_per_day.get(day);
                        let only = games
                            .filter(|ids| ids.len() == 1)
                            .and_then(|ids| ids.iter().next())
                            .and_then(|id| apps.get(id))
                            .map(|app| app.app_name().to_string());
                        (
                            *day,
                            DaySummary {
                                count: *count,
                                games: games.map(HashSet::len).unwrap_or(0),
                                only,
                            },
                        )
                    })
                    .collect();
                heatmap_data.replace(HeatmapData {
                    per_day,
                    total: timeline.total,
                    since: timeline.first.and_then(format_day),
                    years: years.clone(),
                });
                // Splicing empties the model first, and the dropdown reselects
                // as it goes: unblocked, that redraws the grid twice over and
                // flashes "No unlocks yet." through the caption on the way.
                heatmap_years.block_signal(&heatmap_years_handler);
                heatmap_year_model.splice(
                    0,
                    heatmap_year_model.n_items(),
                    &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                heatmap_years.set_visible(years.len() > 1);
                let selected = was
                    .and_then(|year| years.iter().position(|candidate| *candidate == year))
                    .unwrap_or(0) as u32;
                heatmap_years.set_selected(selected);
                heatmap_years.unblock_signal(&heatmap_years_handler);
                draw_heatmap(
                    &heatmap,
                    &heatmap_caption,
                    &heatmap_data.borrow(),
                    selected,
                );

                let totals: HashMap<u32, u32> = apps
                    .iter()
                    .filter(|(_, app)| !app.is_synthetic() && !app.is_junk())
                    .filter(|(_, app)| app.achievements_loaded())
                    .map(|(id, app)| (*id, app.achievement_count()))
                    .collect();
                let curve = timeline::completion_curve(&timeline.chronology, &totals);
                completion_graph.set_data(&curve, today);
                completion_caption.set_label(&match curve.last() {
                    None => tr("No completion to plot yet.").to_string(),
                    Some(point) => tr(
                        "Averaged over the games you have started, so it dips when you start a new one. Now at {percent}%.",
                    )
                    .replace("{percent}", &point.percent.round().to_string()),
                });

                let missing = missing_from_cache(&apps, &timeline.cached_apps);
                cache_banner.set_visible(missing > 0);
                if missing > 0 {
                    cache_banner_label.set_label(
                        &tr("The activity below is missing {count} game(s). Restart Steam to get them back.")
                            .replace("{count}", &missing.to_string()),
                    );
                }

                clear_list(&bursts_list);
                let rows = rows_for(&timeline.bursts, &apps);
                let shown = rows.len().min(BURSTS_MAX_ROWS);
                for row in &rows[..shown] {
                    let mut action = None;
                    let (title, subtitle) = match row {
                        Row::Sitting {
                            start,
                            games,
                            count,
                            apps: sitting,
                        } => {
                            let button = Button::builder()
                                .label(
                                    tr("Select these {games} games")
                                        .replace("{games}", &games.to_string()),
                                )
                                .valign(Align::Center)
                                .build();
                            button.connect_clicked(clone!(
                                #[strong]
                                select_apps,
                                #[strong]
                                sitting,
                                move |_| {
                                    if let Some(select) = select_apps.borrow().as_ref() {
                                        select(&sitting);
                                    }
                                }
                            ));
                            action = Some(button);
                            (
                                tr("{count} achievements across {games} games")
                                    .replace("{count}", &count.to_string())
                                    .replace("{games}", &games.to_string()),
                                tr("{when} — one game after another, in a single sitting")
                                    .replace("{when}", &format_day(*start).unwrap_or_default()),
                            )
                        }
                        Row::Single(reading, burst) => {
                            let app = apps.get(&burst.app_id);
                            let name = app.map(|app| app.app_name()).unwrap_or_else(|| {
                                tr("App {id}").replace("{id}", &burst.app_id.to_string())
                            });
                            if let Some(app) = app {
                                let button = Button::builder()
                                    .icon_name("go-next-symbolic")
                                    .css_classes(["flat"])
                                    .valign(Align::Center)
                                    .tooltip_text(
                                        tr("Open {app}").replace("{app}", &name).as_str(),
                                    )
                                    .build();
                                button.connect_clicked(clone!(
                                    #[strong]
                                    on_open_app,
                                    #[strong]
                                    app,
                                    move |_| on_open_app(&app)
                                ));
                                action = Some(button);
                            }
                            let when = format_day(burst.start).unwrap_or_default();
                            let timing = if burst.span == 0 {
                                tr("{when}, all in the same second").replace("{when}", &when)
                            } else {
                                tr("{when}, inside {span} second(s)")
                                    .replace("{when}", &when)
                                    .replace("{span}", &burst.span.to_string())
                            };
                            (
                                tr("{count} achievements in {app}")
                                    .replace("{count}", &burst.count.to_string())
                                    .replace("{app}", &name),
                                tr("{timing} — {reading}")
                                    .replace("{timing}", &timing)
                                    .replace("{reading}", &reading_text(*reading, app)),
                            )
                        }
                    };
                    bursts_list.append(&list_row(&title, &subtitle, action.as_ref()));
                }
                bursts_list.set_visible(!rows.is_empty());
                bursts_clean.set_visible(rows.is_empty());
                bursts_more.set_visible(rows.len() > shown);
                bursts_more.set_label(
                    &tr("… and {count} more that stand out less.")
                        .replace("{count}", &(rows.len() - shown).to_string()),
                );
            }
        ));
    }
}

fn format_day(unix_seconds: u32) -> Option<String> {
    gtk::glib::DateTime::from_unix_local(i64::from(unix_seconds))
        .ok()?
        .format("%x")
        .ok()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_started_game_weighs_the_same() {
        let stats = LibraryStats {
            started: 4,
            started_rate_sum: 1.0 + 0.5 + 0.25 + 0.5,
            unlocked: 5_000 + 5 + 1 + 1,
            total: 5_000 + 10 + 4 + 2,
            ..LibraryStats::default()
        };

        assert_eq!(completion_percent(&stats), 56);
    }

    #[test]
    fn an_untouched_library_is_not_a_division_by_zero() {
        assert_eq!(completion_percent(&LibraryStats::default()), 0);
    }
}
