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

//! Average completion across the years, as one line.

use super::timeline::CompletionPoint;
use gtk::glib;

glib::wrapper! {
    pub struct CompletionGraph(ObjectSubclass<imp::CompletionGraph>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for CompletionGraph {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl CompletionGraph {
    pub(super) fn set_data(&self, points: &[CompletionPoint], end_day: i32) {
        use gtk::prelude::WidgetExt;
        use gtk::subclass::prelude::ObjectSubclassIsExt;

        let imp = self.imp();
        imp.points.replace(
            points
                .iter()
                .map(|point| (point.day, point.percent))
                .collect(),
        );
        imp.end_day.set(end_day);
        self.queue_draw();
    }
}

mod imp {
    use crate::gui_frontend::i18n::tr;
    use crate::gui_frontend::profile_view::heatmap::scale;
    use crate::gui_frontend::profile_view::timeline::{
        civil_from_days, day_label, days_from_civil,
    };
    use gtk::gdk::RGBA;
    use gtk::glib;
    use gtk::graphene::Point;
    use gtk::gsk::{FillRule, PathBuilder, Stroke};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    const PLOT_HEIGHT: i32 = 132;
    const GAP: i32 = 4;
    const AXIS_WIDTH: i32 = 38;
    const LINE_WIDTH: f32 = 2.0;
    const DOT_RADIUS: f32 = 3.5;

    const GRID_LINES: [f32; 5] = [0.0, 25.0, 50.0, 75.0, 100.0];

    fn y_of(top: f32, percent: f32) -> f32 {
        top + (1.0 - percent / 100.0) * PLOT_HEIGHT as f32
    }

    #[derive(Default)]
    pub struct CompletionGraph {
        pub(super) points: RefCell<Vec<(i32, f32)>>,
        pub(super) end_day: Cell<i32>,
        hover: Cell<Option<f32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CompletionGraph {
        const NAME: &'static str = "SamCompletionGraph";
        type Type = super::CompletionGraph;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for CompletionGraph {
        fn constructed(&self) {
            self.parent_constructed();

            let motion = gtk::EventControllerMotion::new();
            motion.connect_motion(glib::clone!(
                #[weak(rename_to = graph)]
                self.obj(),
                move |_, x, _| graph.imp().set_hover(Some(x as f32))
            ));
            motion.connect_leave(glib::clone!(
                #[weak(rename_to = graph)]
                self.obj(),
                move |_| graph.imp().set_hover(None)
            ));
            self.obj().add_controller(motion);
            self.obj().set_cursor_from_name(Some("crosshair"));
        }
    }

    impl CompletionGraph {
        fn label_height(&self) -> i32 {
            self.obj().create_pango_layout(Some("M")).pixel_size().1
        }

        fn span(&self) -> Option<(i32, i32)> {
            let points = self.points.borrow();
            let first = points.first()?.0;
            let last = self
                .end_day
                .get()
                .max(points.last()?.0)
                .max(first.saturating_add(1));
            Some((first, last))
        }

        fn plot_width(&self) -> i32 {
            (self.obj().width() - AXIS_WIDTH).max(1)
        }

        fn x_of(&self, day: i32, first: i32, last: i32) -> f32 {
            let along = f64::from(day - first) / f64::from(last - first);
            AXIS_WIDTH as f32 + (along * f64::from(self.plot_width())) as f32
        }

        fn reading_at(&self, day: i32) -> Option<f32> {
            let points = self.points.borrow();
            let index = points.partition_point(|(at, _)| *at <= day);
            (index > 0).then(|| points[index - 1].1)
        }

        pub(super) fn set_hover(&self, x: Option<f32>) {
            if self.hover.get() != x {
                self.hover.set(x);
                self.obj().queue_draw();
            }
        }

        fn hovered(&self) -> Option<(i32, f32)> {
            let (first, last) = self.span()?;
            let x = self.hover.get()?;
            if x < AXIS_WIDTH as f32 {
                return None;
            }
            let along = f64::from(x - AXIS_WIDTH as f32) / f64::from(self.plot_width());
            let day = (first + (along * f64::from(last - first)).round() as i32)
                .min(last)
                .min(self.end_day.get());
            Some((day, self.reading_at(day)?))
        }
    }

    impl WidgetImpl for CompletionGraph {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                return (AXIS_WIDTH + 120, AXIS_WIDTH + 640, -1, -1);
            }
            let height = self.label_height() * 2 + GAP * 2 + PLOT_HEIGHT;
            (height, height, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let Some((first, last)) = self.span() else {
                return;
            };
            let widget = self.obj();
            let top = self.label_height() as f32 + GAP as f32;

            let foreground = widget.color();
            // The heatmap's darkest square, so the two read as one history.
            let (red, green, blue) = scale(&foreground)[3];
            let line = RGBA::new(red, green, blue, 1.0);
            let under = RGBA::new(red, green, blue, 0.15);
            let grid = RGBA::new(
                foreground.red(),
                foreground.green(),
                foreground.blue(),
                0.12,
            );
            let dim = RGBA::new(foreground.red(), foreground.green(), foreground.blue(), 0.6);

            let left = AXIS_WIDTH as f32;
            let right = self.x_of(last, first, last);
            for percent in GRID_LINES {
                let y = y_of(top, percent);
                let rule = PathBuilder::new();
                rule.move_to(left, y);
                rule.line_to(right, y);
                snapshot.append_stroke(&rule.to_path(), &Stroke::new(1.0), &grid);

                let layout = widget.create_pango_layout(Some(&format!("{percent:.0}%")));
                let (width, height) = layout.pixel_size();
                snapshot.save();
                snapshot.translate(&Point::new(
                    (AXIS_WIDTH - GAP - width) as f32,
                    y - height as f32 / 2.0,
                ));
                snapshot.append_layout(&layout, &dim);
                snapshot.restore();
            }

            let (first_year, ..) = civil_from_days(first);
            let (last_year, ..) = civil_from_days(last);
            let baseline = y_of(top, 0.0) + GAP as f32;
            let mut previous_label_end = 0.0f32;
            for year in first_year..=last_year {
                let day = days_from_civil(year, 1, 1);
                if day < first || day > last {
                    continue;
                }
                let x = self.x_of(day, first, last);
                let tick = PathBuilder::new();
                tick.move_to(x, y_of(top, 100.0));
                tick.line_to(x, y_of(top, 0.0));
                snapshot.append_stroke(&tick.to_path(), &Stroke::new(1.0), &grid);

                let layout = widget.create_pango_layout(Some(&year.to_string()));
                let width = layout.pixel_size().0 as f32;
                let start = x - width / 2.0;
                if start < previous_label_end {
                    continue;
                }
                previous_label_end = start + width + GAP as f32;
                snapshot.save();
                snapshot.translate(&Point::new(start, baseline));
                snapshot.append_layout(&layout, &dim);
                snapshot.restore();
            }

            let points = self.points.borrow();
            let path = PathBuilder::new();
            let area = PathBuilder::new();
            let start = self.x_of(points[0].0, first, last);
            let mut held_y = y_of(top, points[0].1);
            path.move_to(start, held_y);
            area.move_to(start, y_of(top, 0.0));
            area.line_to(start, held_y);
            // A staircase, not a ramp: completion holds flat until the day it
            // moves, which is the day the readout reports when hovered.
            for (day, percent) in points.iter().skip(1) {
                let x = self.x_of(*day, first, last);
                let y = y_of(top, *percent);
                path.line_to(x, held_y);
                path.line_to(x, y);
                area.line_to(x, held_y);
                area.line_to(x, y);
                held_y = y;
            }
            let (held_day, held) = points[points.len() - 1];
            path.line_to(right, y_of(top, held));
            area.line_to(right, y_of(top, held));
            area.line_to(right, y_of(top, 0.0));
            area.close();

            snapshot.append_fill(&area.to_path(), FillRule::Winding, &under);
            snapshot.append_stroke(&path.to_path(), &Stroke::new(LINE_WIDTH), &line);

            let hovered = self.hovered();
            if let Some((day, percent)) = hovered {
                let x = self.x_of(day, first, last);
                let crosshair = PathBuilder::new();
                crosshair.move_to(x, y_of(top, 100.0));
                crosshair.line_to(x, y_of(top, 0.0));
                snapshot.append_stroke(&crosshair.to_path(), &Stroke::new(1.0), &dim);

                let dot = PathBuilder::new();
                dot.add_circle(&Point::new(x, y_of(top, percent)), DOT_RADIUS);
                snapshot.append_fill(&dot.to_path(), FillRule::Winding, &line);
            }

            let (day, percent) = hovered.unwrap_or((held_day, held));
            let layout = widget.create_pango_layout(Some(
                /* xgettext:no-c-format */
                &tr("{percent}% average on {date}")
                    .replace("{percent}", &percent.round().to_string())
                    .replace("{date}", &day_label(day)),
            ));
            layout.set_width(widget.width() * gtk::pango::SCALE);
            layout.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let width = layout.pixel_size().0 as f32;
            let start = match hovered {
                Some(_) => (self.x_of(day, first, last) - width / 2.0)
                    .clamp(0.0, (widget.width() as f32 - width).max(0.0)),
                None => left,
            };
            snapshot.save();
            snapshot.translate(&Point::new(start, 0.0));
            snapshot.append_layout(&layout, if hovered.is_some() { &foreground } else { &dim });
            snapshot.restore();
        }
    }
}
