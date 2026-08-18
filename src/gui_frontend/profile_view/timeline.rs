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

//! Turns Steam's local unlock records into what the profile page draws: a
//! per-day histogram, and the bursts that stand out of it.
//!
//! Days are *civil* days (local time), not `unix / 86400`: the heatmap's columns
//! are calendar weeks, and an off-by-a-timezone would shear the grid.

use crate::backend::user_unlock_times::UnlockCache;
use gtk::glib::DateTime;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

/// How close together unlocks have to be to form a burst, and how many it takes
/// to be worth showing. A filter for candidates, never a verdict: a game
/// backfilling on first launch grants its catalogue in a frame, and this app's
/// own scheduler can spread a dump out until it reads as ordinary.
const BURST_WINDOW_SECS: u32 = 5;
const BURST_MIN: usize = 10;

const CLUSTER_WINDOW_SECS: u32 = 60;

pub(super) struct Burst {
    pub app_id: u32,
    pub count: usize,
    pub start: u32,
    pub span: u32,
    pub earlier: usize,
    pub later: usize,
    pub ordinary_later: usize,
    pub cluster: usize,
    pub cluster_apps: usize,
}

#[derive(Default)]
pub(super) struct Timeline {
    pub per_day: HashMap<i32, u32>,
    pub apps_per_day: HashMap<i32, HashSet<u32>>,
    pub bursts: Vec<Burst>,
    pub total: usize,
    pub first: Option<u32>,
    pub cached_apps: HashSet<u32>,
    pub chronology: Vec<DayStamp>,
    pub last_full_scan: Option<u64>,
}

pub(super) struct DayStamp {
    pub app_id: u32,
    pub day: i32,
}

pub(super) struct CompletionPoint {
    pub day: i32,
    pub percent: f32,
}

pub(super) fn build(cache: UnlockCache) -> Timeline {
    let UnlockCache {
        mut stamps,
        apps: cached_apps,
        last_full_scan,
    } = cache;
    let total = stamps.len();
    let first = stamps.iter().map(|s| s.unlock_time).min();

    let mut days = LocalDays::default();
    let mut per_day: HashMap<i32, u32> = HashMap::new();
    let mut apps_per_day: HashMap<i32, HashSet<u32>> = HashMap::new();
    let mut chronology: Vec<(u32, DayStamp)> = Vec::with_capacity(stamps.len());
    for stamp in &stamps {
        if let Some(day) = days.day_of(stamp.unlock_time) {
            *per_day.entry(day).or_default() += 1;
            apps_per_day.entry(day).or_default().insert(stamp.app_id);
            chronology.push((
                stamp.unlock_time,
                DayStamp {
                    app_id: stamp.app_id,
                    day,
                },
            ));
        }
    }
    chronology.sort_unstable_by_key(|(unlock_time, _)| *unlock_time);
    let chronology: Vec<DayStamp> = chronology.into_iter().map(|(_, stamp)| stamp).collect();

    stamps.sort_unstable_by_key(|s| (s.app_id, s.unlock_time));
    let mut bursts = Vec::new();
    let mut app_start = 0usize;
    while app_start < stamps.len() {
        let app_id = stamps[app_start].app_id;
        let mut app_end = app_start + 1;
        while app_end < stamps.len() && stamps[app_end].app_id == app_id {
            app_end += 1;
        }
        let app = &stamps[app_start..app_end];

        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut run_start = 0usize;
        for i in 1..=app.len() {
            let breaks =
                i == app.len() || app[i].unlock_time - app[i - 1].unlock_time > BURST_WINDOW_SECS;
            if breaks {
                runs.push((run_start, i));
                run_start = i;
            }
        }

        let mut ordinary_from = vec![0usize; runs.len() + 1];
        for (index, &(start, end)) in runs.iter().enumerate().rev() {
            let ordinary = if end - start < BURST_MIN {
                end - start
            } else {
                0
            };
            ordinary_from[index] = ordinary_from[index + 1] + ordinary;
        }

        for (index, &(start, end)) in runs.iter().enumerate() {
            if end - start < BURST_MIN {
                continue;
            }
            bursts.push(Burst {
                app_id,
                count: end - start,
                start: app[start].unlock_time,
                span: app[end - 1].unlock_time - app[start].unlock_time,
                earlier: start,
                later: app.len() - end,
                ordinary_later: ordinary_from[index + 1],
                cluster: 0,
                cluster_apps: 1,
            });
        }
        app_start = app_end;
    }
    mark_clusters(&mut bursts);
    bursts.sort_unstable_by_key(|burst| std::cmp::Reverse(burst.start));

    Timeline {
        per_day,
        apps_per_day,
        bursts,
        total,
        first,
        cached_apps,
        chronology,
        last_full_scan,
    }
}

/// A game missing from `totals` is left out: it has no denominator yet, so the
/// curve covers a little less than the tile does. It dips as well as climbs:
/// starting a new game adds a fresh 1-of-40 to the average.
pub(super) fn completion_curve(
    chronology: &[DayStamp],
    totals: &HashMap<u32, u32>,
) -> Vec<CompletionPoint> {
    let mut unlocked: HashMap<u32, u32> = HashMap::new();
    let mut rate_sum = 0f64;
    let mut started = 0u32;
    let mut out: Vec<CompletionPoint> = Vec::new();

    for stamp in chronology {
        let Some(&total) = totals.get(&stamp.app_id).filter(|total| **total > 0) else {
            continue;
        };
        let count = unlocked.entry(stamp.app_id).or_insert(0);
        if *count == 0 {
            started += 1;
        }
        // A schema that lost achievements can leave more stamps than it has bits.
        if *count < total {
            *count += 1;
            rate_sum += 1.0 / f64::from(total);
        }

        let percent = (rate_sum / f64::from(started) * 100.0) as f32;
        match out.last_mut() {
            Some(point) if point.day == stamp.day => point.percent = percent,
            _ => out.push(CompletionPoint {
                day: stamp.day,
                percent,
            }),
        }
    }
    out
}

fn mark_clusters(bursts: &mut [Burst]) {
    let mut order: Vec<usize> = (0..bursts.len()).collect();
    order.sort_unstable_by_key(|&i| bursts[i].start);

    let mut group_start = 0usize;
    let mut cluster = 0usize;
    for i in 1..=order.len() {
        let breaks = i == order.len()
            || bursts[order[i]].start - bursts[order[i - 1]].start > CLUSTER_WINDOW_SECS;
        if !breaks {
            continue;
        }
        let group = &order[group_start..i];
        let apps = group
            .iter()
            .map(|&i| bursts[i].app_id)
            .collect::<HashSet<u32>>()
            .len();
        for &i in group {
            bursts[i].cluster = cluster;
            bursts[i].cluster_apps = apps;
        }
        group_start = i;
        cluster += 1;
    }
}

/// Civil day index, in local time, for a stream of stamps.
///
/// `DateTime::from_unix_local` allocates a GObject and resolves the timezone
/// every call: 8 s of the 8.3 s this used to take. A DST shift only lands on a
/// whole hour, so the offset is cached per hour and the rest is arithmetic.
#[derive(Default)]
struct LocalDays {
    offset_by_hour: HashMap<i64, i64>,
}

impl LocalDays {
    /// `None` for a stamp GLib refuses to place on a calendar.
    fn day_of(&mut self, unix_seconds: u32) -> Option<i32> {
        let seconds = i64::from(unix_seconds);
        let offset = match self.offset_by_hour.entry(seconds.div_euclid(3600)) {
            Entry::Occupied(hit) => *hit.get(),
            Entry::Vacant(slot) => {
                let local = DateTime::from_unix_local(seconds).ok()?;
                *slot.insert(local.utc_offset().as_seconds())
            }
        };
        Some((seconds + offset).div_euclid(86_400) as i32)
    }
}

pub(super) fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u32;
    let day_of_year = (153 * if month > 2 { month - 3 } else { month + 9 } + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era as i32 - 719468
}

pub(super) fn civil_from_days(days: i32) -> (i32, u32, u32) {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let day_of_era = (days - era * 146097) as u32;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era as i32 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Today, as a civil day index in the local timezone.
pub(super) fn today() -> i32 {
    let now = DateTime::now_local()
        .unwrap_or_else(|_| DateTime::from_unix_utc(0).expect("the epoch is a representable date"));
    days_from_civil(now.year(), now.month() as u32, now.day_of_month() as u32)
}

/// A civil day in the locale's short date format.
pub(super) fn day_label(day: i32) -> String {
    let (year, month, day_of_month) = civil_from_days(day);
    DateTime::from_local(year, month as i32, day_of_month as i32, 12, 0, 0.0)
        .ok()
        .and_then(|when| when.format("%x").ok())
        .map(|text| text.to_string())
        .unwrap_or_else(|| format!("{year}-{month:02}-{day_of_month:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::user_unlock_times::UnlockStamp;

    #[test]
    fn civil_days_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 3, 1), 11017);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        for day in (-30000..30000).step_by(7) {
            let (y, m, d) = civil_from_days(day);
            assert_eq!(days_from_civil(y, m, d), day);
        }
    }

    #[test]
    fn leap_day_is_a_day() {
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(
            days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
            2
        );
    }

    #[test]
    fn cached_offsets_agree_with_glib() {
        let naive = |unix_seconds: u32| {
            let dt = DateTime::from_unix_local(i64::from(unix_seconds)).unwrap();
            days_from_civil(dt.year(), dt.month() as u32, dt.day_of_month() as u32)
        };
        let mut days = LocalDays::default();
        let start = 1_600_000_000u32;
        for step in 0..(3 * 365 * 72) {
            let unix_seconds = start + step * 1200;
            assert_eq!(
                days.day_of(unix_seconds),
                Some(naive(unix_seconds)),
                "disagreement at {unix_seconds}"
            );
        }
    }

    fn stamps(entries: &[(u32, u32)]) -> UnlockCache {
        UnlockCache {
            stamps: entries
                .iter()
                .map(|&(app_id, unlock_time)| UnlockStamp {
                    app_id,
                    unlock_time,
                })
                .collect(),
            apps: entries.iter().map(|&(app_id, _)| app_id).collect(),
            last_full_scan: None,
        }
    }

    #[test]
    fn a_dump_is_a_burst() {
        let all_at_once: Vec<(u32, u32)> = (0..12).map(|_| (440, 1_700_000_000)).collect();
        let timeline = build(stamps(&all_at_once));
        assert_eq!(timeline.total, 12);
        assert_eq!(timeline.bursts.len(), 1);
        assert_eq!(timeline.bursts[0].count, 12);
        assert_eq!(timeline.bursts[0].span, 0);
    }

    #[test]
    fn playing_is_not_a_burst() {
        let paced: Vec<(u32, u32)> = (0..12).map(|i| (440, 1_700_000_000 + i * 60)).collect();
        assert!(build(stamps(&paced)).bursts.is_empty());

        let split: Vec<(u32, u32)> = (0..12).map(|i| (440 + i % 2, 1_700_000_000)).collect();
        assert!(build(stamps(&split)).bursts.is_empty());
    }

    #[test]
    fn a_burst_stops_at_the_gap() {
        let mut entries: Vec<(u32, u32)> = (0..11).map(|_| (440, 1_700_000_000)).collect();
        entries.extend((0..4).map(|i| (440, 1_700_000_100 + i)));
        let timeline = build(stamps(&entries));
        assert_eq!(timeline.bursts.len(), 1);
        assert_eq!(timeline.bursts[0].count, 11);
    }

    #[test]
    fn a_run_knows_what_surrounds_it() {
        let mut entries: Vec<(u32, u32)> = vec![(440, 1_699_000_000), (440, 1_699_000_500)];
        entries.extend((0..11).map(|_| (440, 1_700_000_000)));
        entries.extend((0..4).map(|i| (440, 1_700_100_000 + i * 900)));
        let timeline = build(stamps(&entries));
        assert_eq!(timeline.bursts.len(), 1);
        assert_eq!(timeline.bursts[0].earlier, 2);
        assert_eq!(timeline.bursts[0].later, 4);
        assert_eq!(timeline.bursts[0].ordinary_later, 4);

        let alone: Vec<(u32, u32)> = (0..11).map(|_| (440, 1_700_000_000)).collect();
        let timeline = build(stamps(&alone));
        assert_eq!(timeline.bursts[0].earlier, 0);
        assert_eq!(timeline.bursts[0].later, 0);
        assert_eq!(timeline.bursts[0].ordinary_later, 0);
    }

    #[test]
    fn more_runs_after_a_run_are_not_ordinary_play() {
        let mut entries: Vec<(u32, u32)> = (0..11).map(|_| (440, 1_700_000_000)).collect();
        entries.extend((0..12).map(|_| (440, 1_700_050_000)));
        entries.extend([(440, 1_700_090_000), (440, 1_700_095_000)]);

        let timeline = build(stamps(&entries));
        let first = timeline
            .bursts
            .iter()
            .find(|burst| burst.start == 1_700_000_000)
            .expect("the first run is a burst");
        assert_eq!(first.later, 14);
        assert_eq!(first.ordinary_later, 2);

        let second = timeline
            .bursts
            .iter()
            .find(|burst| burst.start == 1_700_050_000)
            .expect("the second run is a burst");
        assert_eq!(second.earlier, 11);
        assert_eq!(second.ordinary_later, 2);
    }

    #[test]
    fn games_dumped_together_are_one_sitting() {
        let mut entries = Vec::new();
        for (app, offset) in [(440u32, 0u32), (620, 30), (730, 60)] {
            entries.extend((0..11).map(|i| (app, 1_700_000_000 + offset + i % 3)));
        }
        entries.extend((0..11).map(|_| (570u32, 1_700_003_600u32)));

        let timeline = build(stamps(&entries));
        assert_eq!(timeline.bursts.len(), 4);
        for burst in &timeline.bursts {
            let expected = if burst.app_id == 570 { 1 } else { 3 };
            assert_eq!(burst.cluster_apps, expected, "app {}", burst.app_id);
        }
    }

    /// Day three's second game opens at one of four: (100 + 25) / 2.
    #[test]
    fn starting_a_game_pulls_the_average_down() {
        const DAY: u32 = 86_400;
        let entries = [
            (440, 1_700_000_000),
            (440, 1_700_000_000 + DAY),
            (620, 1_700_000_000 + 2 * DAY),
        ];
        let totals = HashMap::from([(440u32, 2u32), (620, 4)]);

        let curve = completion_curve(&build(stamps(&entries)).chronology, &totals);
        let percents: Vec<u32> = curve.iter().map(|p| p.percent.round() as u32).collect();
        assert_eq!(percents, vec![50, 100, 63]);
        assert_eq!(curve[1].day - curve[0].day, 1);
    }

    #[test]
    fn an_unmeasured_game_is_not_averaged_in() {
        let entries = [(440, 1_700_000_000), (999, 1_700_100_000)];
        let totals = HashMap::from([(440u32, 4u32)]);

        let curve = completion_curve(&build(stamps(&entries)).chronology, &totals);
        assert_eq!(curve.len(), 1);
        assert_eq!(curve[0].percent.round() as u32, 25);
    }

    #[test]
    fn nothing_measured_is_a_flat_nothing() {
        let curve = completion_curve(
            &build(stamps(&[(440, 1_700_000_000)])).chronology,
            &HashMap::new(),
        );
        assert!(curve.is_empty());
    }

    #[test]
    fn one_game_in_pieces_is_still_one_game() {
        let mut entries: Vec<(u32, u32)> = (0..11).map(|_| (440, 1_700_000_000)).collect();
        entries.extend((0..11).map(|_| (440, 1_700_000_030)));
        let timeline = build(stamps(&entries));
        assert_eq!(timeline.bursts.len(), 2);
        assert!(timeline.bursts.iter().all(|b| b.cluster_apps == 1));
    }
}
