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

//! A year of unlock activity as one square per day, in calendar-week columns.

use crate::gui_frontend::i18n::tr_noop;
use gtk::glib;
use std::collections::HashMap;

/// Columns in the grid; 53 weeks is the fewest that always covers a full year.
const WEEKS: i32 = 53;

/// TRANSLATORS: month abbreviations along the top of the activity heatmap; keep
/// them to three or four characters so the columns stay readable.
/// `%b` is not usable here: it renders in the system's LC_TIME, ignoring the
/// language chosen in Settings.
const MONTHS: [&str; 12] = [
    tr_noop("Jan"),
    tr_noop("Feb"),
    tr_noop("Mar"),
    tr_noop("Apr"),
    tr_noop("May"),
    tr_noop("Jun"),
    tr_noop("Jul"),
    tr_noop("Aug"),
    tr_noop("Sep"),
    tr_noop("Oct"),
    tr_noop("Nov"),
    tr_noop("Dec"),
];

pub(super) use imp::grid_start;

/// Picked by foreground luminance, which is what keeps this readable under a
/// theme we have never seen. The completion graph draws its line from the same
/// palette, so the two read as one history.
const LIGHT_SCALE: [(f32, f32, f32); 4] = [
    (0.607, 0.913, 0.658),
    (0.250, 0.768, 0.388),
    (0.188, 0.631, 0.305),
    (0.129, 0.431, 0.223),
];
const DARK_SCALE: [(f32, f32, f32); 4] = [
    (0.054, 0.266, 0.160),
    (0.000, 0.427, 0.196),
    (0.149, 0.650, 0.254),
    (0.223, 0.827, 0.325),
];

pub(super) fn scale(foreground: &gtk::gdk::RGBA) -> [(f32, f32, f32); 4] {
    let dark =
        0.2126 * foreground.red() + 0.7152 * foreground.green() + 0.0722 * foreground.blue() > 0.5;
    if dark { DARK_SCALE } else { LIGHT_SCALE }
}

glib::wrapper! {
    pub struct Heatmap(ObjectSubclass<imp::Heatmap>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Heatmap {
    fn default() -> Self {
        glib::Object::new()
    }
}

#[derive(Clone, Default)]
pub(super) struct DaySummary {
    pub count: u32,
    pub games: usize,
    pub only: Option<String>,
}

impl Heatmap {
    pub(super) fn set_data(&self, per_day: &HashMap<i32, DaySummary>, end_day: i32) {
        use gtk::prelude::WidgetExt;
        use gtk::subclass::prelude::ObjectSubclassIsExt;

        let imp = self.imp();
        let start = imp::grid_start(end_day);
        let days: Vec<DaySummary> = (0..WEEKS * 7)
            .map(|i| {
                let day = start + i;
                if day > end_day {
                    return DaySummary::default();
                }
                per_day.get(&day).cloned().unwrap_or_default()
            })
            .collect();
        imp.days.replace(days);
        // Only a year switch moves the square out from under the pointer; the
        // periodic reload must not blank the readout every few seconds.
        if imp.end_day.replace(end_day) != end_day {
            imp.set_hover(None);
        }
        self.queue_draw();
    }
}

mod imp {
    use super::{DaySummary, WEEKS};
    use crate::gui_frontend::i18n::tr;
    use crate::gui_frontend::profile_view::timeline::{civil_from_days, day_label};
    use gtk::gdk::RGBA;
    use gtk::glib;
    use gtk::graphene::{Point, Rect};
    use gtk::gsk::RoundedRect;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    const GAP: i32 = 3;
    const MIN_CELL: i32 = 7;
    const MAX_CELL: i32 = 14;

    const LEVELS: [u32; 4] = [1, 3, 6, 11];

    const GROW_PIXELS: f32 = 4.0;
    const GROW_SECONDS: f32 = 0.09;
    const GROW_CARRY: f32 = 0.55;

    pub(crate) fn grid_start(end_day: i32) -> i32 {
        end_day - (WEEKS - 1) * 7 - weekday(end_day)
    }

    fn weekday(day: i32) -> i32 {
        (day + 3).rem_euclid(7)
    }

    fn cell_size(width: i32) -> i32 {
        ((width + GAP) / WEEKS - GAP).clamp(MIN_CELL, MAX_CELL)
    }

    fn level(count: u32) -> Option<usize> {
        if count == 0 {
            return None;
        }
        Some(LEVELS.iter().filter(|&&floor| count >= floor).count() - 1)
    }

    #[derive(Default)]
    pub struct Heatmap {
        pub(super) days: RefCell<Vec<DaySummary>>,
        pub(super) end_day: Cell<i32>,
        hover: Cell<Option<(i32, i32)>>,
        grow: Cell<f32>,
        ticking: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Heatmap {
        const NAME: &'static str = "SamHeatmap";
        type Type = super::Heatmap;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Heatmap {
        /// A readout that follows the pointer, not a tooltip you have to wait out.
        fn constructed(&self) {
            self.parent_constructed();

            let motion = gtk::EventControllerMotion::new();
            motion.connect_motion(glib::clone!(
                #[weak(rename_to = grid)]
                self.obj(),
                move |_, x, y| {
                    let cell = grid.imp().cell_at(x, y);
                    grid.imp().set_hover(cell);
                }
            ));
            motion.connect_leave(glib::clone!(
                #[weak(rename_to = grid)]
                self.obj(),
                move |_| grid.imp().set_hover(None)
            ));
            self.obj().add_controller(motion);
        }
    }

    impl Heatmap {
        fn label_height(&self) -> i32 {
            self.obj().create_pango_layout(Some("M")).pixel_size().1 + GAP
        }

        fn cell_at(&self, x: f64, y: f64) -> Option<(i32, i32)> {
            if self.days.borrow().is_empty() {
                return None;
            }
            let cell = cell_size(self.obj().width());
            let step = cell + GAP;
            let top = self.label_height();
            let column = (x as i32) / step;
            let row = (y as i32 - top) / step;
            if y < f64::from(top)
                || !(0..WEEKS).contains(&column)
                || !(0..7).contains(&row)
                || (x as i32) % step >= cell
                || (y as i32 - top) % step >= cell
            {
                return None;
            }
            (self.day_at(column, row) <= self.end_day.get()).then_some((column, row))
        }

        fn day_at(&self, column: i32, row: i32) -> i32 {
            grid_start(self.end_day.get()) + column * 7 + row
        }

        fn summary_at(&self, column: i32, row: i32) -> DaySummary {
            self.days
                .borrow()
                .get((column * 7 + row) as usize)
                .cloned()
                .unwrap_or_default()
        }

        pub(super) fn set_hover(&self, cell: Option<(i32, i32)>) {
            if self.hover.get() == cell {
                return;
            }
            // Knocked back rather than reset, so sweeping across the grid gives
            // each square a small nudge instead of a flicker down from full size.
            if cell.is_some() {
                self.grow.set(self.grow.get().min(GROW_CARRY));
            }
            self.hover.set(cell);
            self.start_growing();
            self.obj().queue_draw();
        }

        fn start_growing(&self) {
            if self.ticking.replace(true) {
                return;
            }
            // `add_tick_callback` takes an `Fn`, so this cannot be a local.
            let last_frame = Cell::new(0i64);
            self.obj().add_tick_callback(move |grid, clock| {
                let imp = grid.imp();
                let now = clock.frame_time();
                let previous = last_frame.replace(now);
                let elapsed = if previous == 0 {
                    0.0
                } else {
                    (now - previous) as f32 / 1_000_000.0
                };

                let target = if imp.hover.get().is_some() { 1.0 } else { 0.0 };
                let step = elapsed / GROW_SECONDS;
                let grow = imp.grow.get();
                let next = if grow < target {
                    (grow + step).min(target)
                } else {
                    (grow - step).max(target)
                };
                imp.grow.set(next);
                grid.queue_draw();

                if (next - target).abs() < 0.001 {
                    imp.grow.set(target);
                    imp.ticking.set(false);
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
        }
    }

    impl WidgetImpl for Heatmap {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                return (
                    WEEKS * (MIN_CELL + GAP) - GAP,
                    WEEKS * (MAX_CELL + GAP) - GAP,
                    -1,
                    -1,
                );
            }
            let cell = if for_size < 0 {
                MAX_CELL
            } else {
                cell_size(for_size)
            };
            let label = self.label_height();
            let height = label * 2 + 7 * (cell + GAP) - GAP;
            (height, height, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let cell = cell_size(widget.width());
            let step = cell + GAP;
            let top = self.label_height();

            let foreground = widget.color();
            let scale = super::scale(&foreground);
            let empty = RGBA::new(
                foreground.red(),
                foreground.green(),
                foreground.blue(),
                0.10,
            );
            let dim = RGBA::new(foreground.red(), foreground.green(), foreground.blue(), 0.6);

            let days = self.days.borrow();
            let start = grid_start(self.end_day.get());
            let mut last_label_column = -3;
            for column in 0..WEEKS {
                let (_, month, day_of_month) = civil_from_days(start + column * 7);
                if day_of_month <= 7 && column - last_label_column >= 3 {
                    last_label_column = column;
                    let label = month
                        .checked_sub(1)
                        .and_then(|index| super::MONTHS.get(index as usize))
                        .map(|name| tr(name).to_string())
                        .unwrap_or_else(|| month.to_string());
                    let layout = widget.create_pango_layout(Some(&label));
                    snapshot.save();
                    snapshot.translate(&Point::new((column * step) as f32, 0.0));
                    snapshot.append_layout(&layout, &dim);
                    snapshot.restore();
                }

                for row in 0..7 {
                    let day = start + column * 7 + row;
                    if day > self.end_day.get() {
                        break;
                    }
                    // Drawn last instead: grown, it overlaps its neighbours.
                    if self.hover.get() == Some((column, row)) {
                        continue;
                    }
                    let count = days
                        .get((column * 7 + row) as usize)
                        .map(|day| day.count)
                        .unwrap_or(0);
                    let color = match level(count) {
                        Some(i) => {
                            let (r, g, b) = scale[i];
                            RGBA::new(r, g, b, 1.0)
                        }
                        None => empty,
                    };
                    let rect = Rect::new(
                        (column * step) as f32,
                        (top + row * step) as f32,
                        cell as f32,
                        cell as f32,
                    );
                    snapshot.push_rounded_clip(&RoundedRect::from_rect(rect, 2.0));
                    snapshot.append_color(&color, &rect);
                    snapshot.pop();
                }
            }

            let Some((column, row)) = self.hover.get() else {
                return;
            };
            let summary = self.summary_at(column, row);
            let count = summary.count;
            let color = match level(count) {
                Some(i) => {
                    let (red, green, blue) = scale[i];
                    RGBA::new(red, green, blue, 1.0)
                }
                None => empty,
            };
            let swell = self.grow.get() * GROW_PIXELS;
            let rect = Rect::new(
                (column * step) as f32 - swell / 2.0,
                (top + row * step) as f32 - swell / 2.0,
                cell as f32 + swell,
                cell as f32 + swell,
            );
            snapshot.push_rounded_clip(&RoundedRect::from_rect(rect, 2.0));
            snapshot.append_color(&color, &rect);
            snapshot.pop();

            let date = day_label(self.day_at(column, row));
            let text = match (count, summary.games, summary.only.as_deref()) {
                (0, ..) => tr("No unlocks on {date}").replace("{date}", &date),
                (_, 1, Some(game)) => tr("{count} unlock(s) in {game} on {date}")
                    .replace("{count}", &count.to_string())
                    .replace("{game}", game)
                    .replace("{date}", &date),
                (_, games, _) if games > 1 => {
                    tr("{count} unlock(s) across {games} games on {date}")
                        .replace("{count}", &count.to_string())
                        .replace("{games}", &games.to_string())
                        .replace("{date}", &date)
                }
                _ => tr("{count} unlock(s) on {date}")
                    .replace("{count}", &count.to_string())
                    .replace("{date}", &date),
            };
            let layout = widget.create_pango_layout(Some(&text));
            // Nothing clips this, and a game name can be as long as Steam allows.
            layout.set_width(widget.width() * gtk::pango::SCALE);
            layout.set_ellipsize(gtk::pango::EllipsizeMode::End);
            snapshot.save();
            snapshot.translate(&Point::new(0.0, (top + 7 * step - GAP) as f32 + GAP as f32));
            snapshot.append_layout(&layout, &foreground);
            snapshot.restore();
        }
    }
}
