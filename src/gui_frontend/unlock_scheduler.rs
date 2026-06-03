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

use crate::dev_println;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{Request, SetAchievement};
use crate::utils::format::format_seconds_to_hh_mm_ss;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub const SPACING_EVEN: &str = "even";
pub const SPACING_RANDOM: &str = "random";

const MIN_GAP_MS: u64 = 2_000;

pub fn compute_unlock_times_ms(count: usize, total_ms: u64, spacing: &str) -> Vec<u64> {
    if count == 0 {
        return Vec::new();
    }
    let step = total_ms / count as u64;
    let linear: Vec<u64> = (0..count).map(|i| (i as u64 + 1) * step).collect();

    if spacing != SPACING_RANDOM || count <= 2 {
        return linear;
    }

    let first = linear[0];
    let last = *linear.last().unwrap();
    let interior_n = count - 2;

    let mut rng = SeededRng::from_time();
    let mut interior: Vec<u64> = (0..interior_n)
        .map(|_| first + rng.next_u64_in(last.saturating_sub(first)))
        .collect();
    interior.sort_unstable();

    let mut out = Vec::with_capacity(count);
    out.push(first);
    out.extend(interior);
    out.push(last);
    enforce_min_gap(&mut out, MIN_GAP_MS, total_ms);
    out
}

fn enforce_min_gap(times: &mut [u64], min_gap: u64, total_ms: u64) {
    for i in 1..times.len() {
        let prev = times[i - 1];
        if times[i] < prev + min_gap {
            times[i] = (prev + min_gap).min(total_ms);
        }
    }
    if let Some(last) = times.last_mut()
        && *last > total_ms
    {
        *last = total_ms;
    }
}

pub async fn run_timed_unlock(
    app_id: u32,
    achievements: Vec<GAchievementObject>,
    times_ms: Vec<u64>,
    timed_raw_model: ListStore,
    cancelled: Arc<AtomicBool>,
) {
    debug_assert_eq!(achievements.len(), times_ms.len());
    if achievements.is_empty() {
        return;
    }

    timed_raw_model.remove_all();
    timed_raw_model.extend_from_slice(&achievements);

    let smallest_gap = times_ms
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .min()
        .unwrap_or(1_000);
    let refresh_ms = smallest_gap.clamp(50, 1_000);
    let refresh_duration = std::time::Duration::from_millis(refresh_ms);

    let start_time = std::time::Instant::now();
    let mut next_index = 0usize;

    while next_index < achievements.len() {
        let tick_start = std::time::Instant::now();

        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            dev_println!("CLIENT", "Timed unlock task cancelled");
            timed_raw_model.remove_all();
            return;
        }

        let elapsed_ms = start_time.elapsed().as_millis() as u64;

        while next_index < achievements.len() && elapsed_ms >= times_ms[next_index] {
            let achievement = &achievements[next_index];
            dev_println!("CLIENT", "Timed unlock of {}", achievement.name());
            achievement.set_is_achieved(true);

            let achievement_id = achievement.id();
            let result = spawn_blocking(move || {
                SetAchievement {
                    app_id,
                    achievement_id,
                    unlocked: true,
                    store: true,
                }
                .request()
            })
            .await;

            match result {
                Ok(response) => dev_println!("CLIENT", "Achievement result: {:?}", response),
                Err(e) => eprintln!("[CLIENT] Achievement failed: {:?}", e),
            }

            next_index += 1;
        }

        for (i, ach) in achievements.iter().enumerate() {
            let remaining_ms = times_ms[i].saturating_sub(elapsed_ms);
            let remaining_seconds = remaining_ms / 1000;
            if remaining_seconds == 0 {
                ach.set_time_until_unlock(tr("OK").as_str());
            } else {
                ach.set_time_until_unlock(format_seconds_to_hh_mm_ss(remaining_seconds as usize));
            }
        }

        let work = tick_start.elapsed();
        let sleep = refresh_duration.saturating_sub(work.min(refresh_duration));
        glib::timeout_future(sleep).await;
    }

    dev_println!("CLIENT", "Timed unlock task finished");
}

pub fn unlock_all_immediately(app_id: u32, achievements: &[GAchievementObject]) {
    use crate::gui_frontend::request::StoreStatsAndAchievements;

    for ach in achievements {
        let res = SetAchievement {
            app_id,
            achievement_id: ach.id(),
            unlocked: true,
            store: false,
        }
        .request();
        if let Err(e) = res {
            eprintln!("[CLIENT] Failed to set achievement: {:?}", e);
        }
    }

    if let Err(e) = (StoreStatsAndAchievements { app_id }.request()) {
        eprintln!("[CLIENT] Failed to store stats and achievements: {:?}", e);
    }
}

struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn from_time() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Self { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_u64_in(&mut self, exclusive_upper: u64) -> u64 {
        if exclusive_upper == 0 {
            return 0;
        }
        self.next_u64() % exclusive_upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_spacing_is_uniform() {
        let times = compute_unlock_times_ms(5, 1000, SPACING_EVEN);
        assert_eq!(times, vec![200, 400, 600, 800, 1000]);
    }

    #[test]
    fn random_spacing_pins_first_and_last() {
        let times = compute_unlock_times_ms(5, 10_000, SPACING_RANDOM);
        assert_eq!(times.len(), 5);
        assert_eq!(times[0], 2_000);
        assert_eq!(times[4], 10_000);
        for w in times.windows(2) {
            assert!(w[1] >= w[0]);
        }
    }

    #[test]
    fn random_spacing_with_two_is_linear() {
        let times = compute_unlock_times_ms(2, 10_000, SPACING_RANDOM);
        assert_eq!(times, vec![5_000, 10_000]);
    }

    #[test]
    fn random_spacing_with_one_is_linear() {
        let times = compute_unlock_times_ms(1, 10_000, SPACING_RANDOM);
        assert_eq!(times, vec![10_000]);
    }
}
