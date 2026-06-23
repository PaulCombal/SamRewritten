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
use crate::backend::user_unlock_times::Friend;
use crate::gui_frontend::MainApplication;
use crate::gui_frontend::friend_picker::open_friend_picker;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::gobjects::mode_state::{GUnlockModeState, MODE_COPY_TIMING};
use crate::gui_frontend::i18n::tr;
use crate::gui_frontend::request::{GetFriendUnlockTimes, GetFriends, GetUserAvatar, Request};
use crate::gui_frontend::unlock_queue::UnlockQueue;
use crate::gui_frontend::unlock_scheduler::{compute_copy_timing_ms, run_timed_unlock};
use crate::gui_frontend::widgets::shimmer_image::ShimmerImage;
use crate::utils::format::format_seconds_to_hh_mm_ss;
use crate::utils::ipc_types::SamError;
use gtk::Stack;
use gtk::gio::{ListStore, spawn_blocking};
use gtk::glib::{self, MainContext, clone};
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Fetch a friend's achieved timeline (id, unix time), sorted, for `app_id`.
/// Errors (e.g. a private profile) propagate so the caller can react.
async fn fetch_copy_source(app_id: u32, steam_id64: u64) -> Result<Vec<(String, u32)>, SamError> {
    let list = spawn_blocking(move || {
        GetFriendUnlockTimes {
            app_id,
            friend: steam_id64.to_string(),
        }
        .request()
    })
    .await
    .expect("spawn_blocking task panicked")?;
    let mut source: Vec<(String, u32)> = list
        .into_iter()
        .filter(|a| a.achieved && a.unlock_time.is_some())
        .map(|a| (a.api_name, a.unlock_time.unwrap()))
        .collect();
    source.sort_by_key(|(_, t)| *t);
    Ok(source)
}

/// Wire up the copy-timing mode: friend selection, plan derivation against the
/// live model, the preview/Advanced controls, and Start. Owns the copy-mode
/// state (the friend's source timeline, the derived plan, and the restore latch);
/// `copy_queue` is shared with the assembler, which hides it on mode switches.
#[allow(clippy::too_many_arguments)]
pub(super) fn install_copy_mode(
    copy: &CopyControls,
    copy_queue: &Rc<UnlockQueue>,
    settings: &gtk::gio::Settings,
    mode_state: &Rc<GUnlockModeState>,
    app_id: &Rc<Cell<Option<u32>>>,
    raw_model: &ListStore,
    timed_raw_model: &ListStore,
    cancelled_task: &Arc<AtomicBool>,
    achievement_views_stack: &Stack,
    application: &MainApplication,
) {
    let copy_queue = copy_queue.clone();
    let mode_state = mode_state.clone();
    let app_id = app_id.clone();
    let settings = settings.clone();
    let raw_model = raw_model.clone();
    let timed_raw_model = timed_raw_model.clone();
    let cancelled_task = cancelled_task.clone();
    let achievement_views_stack = achievement_views_stack.clone();
    let application = application.clone();

    // The friend's filtered, ordered (achievement id, unlock unix time) plan.
    let copy_plan: Rc<RefCell<Vec<(String, u32)>>> = Rc::new(RefCell::new(Vec::new()));

    let refresh_copy_preview: Rc<dyn Fn()> = {
        let copy_plan = Rc::clone(&copy_plan);
        let raw_model = raw_model.clone();
        let preview_label = copy.preview_label.clone();
        let start_button = copy.start_button.clone();
        let max_gap_spin = copy.max_gap_spin.clone();
        let first_delay_spin = copy.first_delay_spin.clone();
        Rc::new(move || {
            let plan = copy_plan.borrow();
            if plan.is_empty() {
                preview_label.set_label(&tr("No friend loaded"));
                start_button.set_sensitive(false);
                return;
            }
            let times: Vec<u32> = plan.iter().map(|(_, t)| *t).collect();
            let max_gap_s = (max_gap_spin.value_as_int().max(0) as u64) * 60;
            let first_delay_s = (first_delay_spin.value_as_int().max(0) as u64) * 60;
            let offsets = compute_copy_timing_ms(&times, max_gap_s, first_delay_s);
            let total_s = offsets.last().copied().unwrap_or(0) / 1000;
            preview_label.set_label(
                &tr("{count} achievements over {time}")
                    .replace("{count}", &plan.len().to_string())
                    .replace("{time}", &format_seconds_to_hh_mm_ss(total_s as usize)),
            );
            start_button.set_sensitive(true);

            // Stamp each staged achievement's time-before-unlock (hh:mm) for its row.
            use std::collections::HashMap;
            let offset_by_id: HashMap<&str, u64> = plan
                .iter()
                .enumerate()
                .map(|(i, (id, _))| (id.as_str(), offsets[i] / 1000))
                .collect();
            for obj in raw_model.into_iter().flatten() {
                if let Ok(a) = obj.downcast::<GAchievementObject>() {
                    let id = a.id();
                    if let Some(&s) = offset_by_id.get(id.as_str()) {
                        a.set_time_until_unlock(format!("{:02}:{:02}", s / 3600, (s % 3600) / 60));
                    }
                }
            }
        })
    };
    refresh_copy_preview();
    {
        let f = Rc::clone(&refresh_copy_preview);
        copy.max_gap_spin.connect_value_notify(move |_| f());
    }
    {
        let f = Rc::clone(&refresh_copy_preview);
        copy.first_delay_spin.connect_value_notify(move |_| f());
    }

    // The friend's full achieved timeline, re-derived against the live model each
    // time it changes, so resetting the game re-expands the plan and its timing.
    let copy_source: Rc<RefCell<Vec<(String, u32)>>> = Rc::new(RefCell::new(Vec::new()));
    let recompute_plan: Rc<dyn Fn()> = {
        let copy_source = Rc::clone(&copy_source);
        let copy_plan = Rc::clone(&copy_plan);
        let copy_queue = Rc::clone(&copy_queue);
        let raw_model = raw_model.clone();
        let refresh_copy_preview = Rc::clone(&refresh_copy_preview);
        Rc::new(move || {
            use std::collections::HashMap;
            let mut model: HashMap<String, (bool, i32)> = HashMap::new();
            for obj in raw_model.into_iter().flatten() {
                if let Ok(a) = obj.downcast::<GAchievementObject>() {
                    model.insert(a.id(), (a.is_achieved(), a.permission()));
                }
            }
            // Keep the friend's achievements we don't already own and can write.
            let plan: Vec<(String, u32)> = copy_source
                .borrow()
                .iter()
                .filter(|(id, _)| matches!(model.get(id), Some(&(false, 0))))
                .cloned()
                .collect();
            let ids: Vec<String> = plan.iter().map(|(id, _)| id.clone()).collect();
            *copy_plan.borrow_mut() = plan;
            copy_queue.set_order(&raw_model, &ids);
            refresh_copy_preview();
        })
    };

    // Remember a loaded friend's timeline, derive the plan, set the avatar, and
    // persist the selection (by SteamID) for next run.
    let apply_friend: Rc<dyn Fn(Friend, Vec<(String, u32)>)> = {
        let settings = settings.clone();
        let copy_source = Rc::clone(&copy_source);
        let recompute_plan = Rc::clone(&recompute_plan);
        let avatar_button = copy.avatar_button.clone();
        Rc::new(move |friend: Friend, source: Vec<(String, u32)>| {
            let _ = settings.set_string("copy-timing-friend", &friend.steam_id64.to_string());
            avatar_button.set_tooltip_text(Some(&friend.name));

            // A friend carries a CDN url; a pasted custom SteamID has none, so
            // fetch its avatar natively from Steam instead.
            if friend.avatar_url.is_empty() {
                let steam_id64 = friend.steam_id64;
                let handle = spawn_blocking(move || GetUserAvatar { steam_id64 }.request());
                MainContext::default().spawn_local(clone!(
                    #[weak]
                    avatar_button,
                    async move {
                        let avatar = handle.await.expect("spawn_blocking task panicked");
                        match avatar {
                            Ok(Some(img)) => {
                                let shimmer = ShimmerImage::new();
                                shimmer.set_size_request(22, 22);
                                shimmer.set_rgba(img.width as i32, img.height as i32, &img.rgba);
                                avatar_button.set_child(Some(&shimmer));
                            }
                            _ => {
                                let icon = gtk::Image::from_icon_name("avatar-default-symbolic");
                                icon.set_pixel_size(22);
                                avatar_button.set_child(Some(&icon));
                            }
                        }
                    }
                ));
            } else {
                let shimmer = ShimmerImage::new();
                shimmer.set_size_request(22, 22);
                shimmer.set_url(friend.avatar_url.as_str());
                avatar_button.set_child(Some(&shimmer));
            }

            *copy_source.borrow_mut() = source;
            recompute_plan();
        })
    };

    let clear_friend: Rc<dyn Fn()> = {
        let settings = settings.clone();
        let copy_source = Rc::clone(&copy_source);
        let recompute_plan = Rc::clone(&recompute_plan);
        let avatar_button = copy.avatar_button.clone();
        Rc::new(move || {
            let _ = settings.set_string("copy-timing-friend", "");
            let icon = gtk::Image::from_icon_name("avatar-default-symbolic");
            icon.set_pixel_size(22);
            avatar_button.set_child(Some(&icon));
            avatar_button
                .set_tooltip_text(Some(tr("Choose a friend to copy timing from").as_str()));
            copy_source.borrow_mut().clear();
            recompute_plan();
        })
    };

    // Restore the last-selected friend (saved by SteamID) once per session.
    let copy_restored = Rc::new(Cell::new(false));
    let try_restore_friend: Rc<dyn Fn()> = {
        let settings = settings.clone();
        let app_id = app_id.clone();
        let copy_source = Rc::clone(&copy_source);
        let copy_restored = copy_restored.clone();
        let apply_friend = Rc::clone(&apply_friend);
        Rc::new(move || {
            if copy_restored.get() || !copy_source.borrow().is_empty() {
                return;
            }
            let Ok(steam_id64) = settings.string("copy-timing-friend").parse::<u64>() else {
                return;
            };
            let Some(app_id_val) = app_id.get() else {
                return;
            };
            copy_restored.set(true);

            // A private profile is forgotten so we stop retrying it; a transient
            // error is left for next time.
            let apply_friend = Rc::clone(&apply_friend);
            let settings = settings.clone();
            let handle = spawn_blocking(move || GetFriends.request());
            MainContext::default().spawn_local(async move {
                let friend = handle
                    .await
                    .expect("spawn_blocking task panicked")
                    .ok()
                    .and_then(|friends| friends.into_iter().find(|f| f.steam_id64 == steam_id64))
                    .unwrap_or(Friend {
                        name: steam_id64.to_string(),
                        steam_id64,
                        avatar_url: String::new(),
                    });
                match fetch_copy_source(app_id_val, steam_id64).await {
                    Ok(source) => apply_friend(friend, source),
                    Err(SamError::ProfilePrivate) => {
                        let _ = settings.set_string("copy-timing-friend", "");
                    }
                    Err(_) => {}
                }
            });
        })
    };

    // The model is rebuilt on refresh/reset/cancel; re-derive the plan against it
    // so staged positions and timing come back, or restore the saved friend.
    raw_model.connect_items_changed(clone!(
        #[strong]
        mode_state,
        #[strong]
        recompute_plan,
        #[strong]
        try_restore_friend,
        #[strong]
        copy_source,
        move |_, _, _, _| {
            if mode_state.mode() == MODE_COPY_TIMING {
                if copy_source.borrow().is_empty() {
                    try_restore_friend();
                } else {
                    recompute_plan();
                }
            }
        }
    ));

    // Switching modes clears the queue; re-stage (or restore) on returning to copy.
    mode_state.connect_mode_notify(clone!(
        #[strong]
        recompute_plan,
        #[strong]
        try_restore_friend,
        #[strong]
        copy_source,
        move |state| {
            if state.mode() == MODE_COPY_TIMING {
                if copy_source.borrow().is_empty() {
                    try_restore_friend();
                } else {
                    recompute_plan();
                }
            }
        }
    ));

    copy.avatar_button.connect_clicked(clone!(
        #[strong]
        app_id,
        #[strong]
        apply_friend,
        #[strong]
        clear_friend,
        #[strong]
        application,
        #[strong]
        settings,
        move |_| {
            let Some(app_id_val) = app_id.get() else {
                return;
            };

            let has_selection = !settings.string("copy-timing-friend").is_empty();
            let handle = spawn_blocking(move || GetFriends.request());
            MainContext::default().spawn_local(clone!(
                #[strong]
                apply_friend,
                #[strong]
                clear_friend,
                #[strong]
                application,
                async move {
                    let friends = handle
                        .await
                        .expect("spawn_blocking task panicked")
                        .unwrap_or_default();
                    let parent = application.active_window();
                    let clear_friend = Rc::clone(&clear_friend);
                    // The picker drives loading: fetch the timeline, apply on
                    // success (closes the picker), or surface the error in-place.
                    open_friend_picker(
                        parent.as_ref(),
                        friends,
                        has_selection,
                        move || clear_friend(),
                        move |friend| {
                            let apply_friend = Rc::clone(&apply_friend);
                            async move {
                                let source =
                                    fetch_copy_source(app_id_val, friend.steam_id64).await?;
                                apply_friend(friend, source);
                                Ok(())
                            }
                        },
                    );
                }
            ));
        }
    ));

    // Start: replay the friend's cadence from now via the shared timed scheduler.
    copy.start_button.connect_clicked(clone!(
        #[strong]
        app_id,
        #[strong]
        copy_plan,
        #[strong]
        cancelled_task,
        #[strong]
        timed_raw_model,
        #[weak(rename_to = raw_model)]
        raw_model,
        #[weak(rename_to = max_gap_spin)]
        copy.max_gap_spin,
        #[weak(rename_to = first_delay_spin)]
        copy.first_delay_spin,
        #[weak(rename_to = achievement_views_stack)]
        achievement_views_stack,
        move |_| {
            let plan = copy_plan.borrow().clone();
            if plan.is_empty() {
                return;
            }
            let Some(app_id_val) = app_id.get() else {
                return;
            };
            let ids: Vec<String> = plan.iter().map(|(id, _)| id.clone()).collect();
            let achievements = super::resolve_queue_to_objects(&raw_model, &ids);
            if achievements.len() != plan.len() {
                eprintln!("[CLIENT] Copy plan/model mismatch; aborting");
                return;
            }
            let times: Vec<u32> = plan.iter().map(|(_, t)| *t).collect();
            let max_gap_s = (max_gap_spin.value_as_int().max(0) as u64) * 60;
            let first_delay_s = (first_delay_spin.value_as_int().max(0) as u64) * 60;
            let times_ms = compute_copy_timing_ms(&times, max_gap_s, first_delay_s);

            cancelled_task.store(false, std::sync::atomic::Ordering::Relaxed);
            MainContext::default().spawn_local(clone!(
                #[strong]
                timed_raw_model,
                #[strong]
                cancelled_task,
                async move {
                    run_timed_unlock(
                        app_id_val,
                        achievements,
                        times_ms,
                        timed_raw_model,
                        cancelled_task,
                    )
                    .await;
                }
            ));
            achievement_views_stack.set_visible_child_name("automatic");
        }
    ));
}
