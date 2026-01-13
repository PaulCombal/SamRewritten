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

use crate::gui_frontend::MainApplication;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use gtk::gio::ListStore;
use gtk::prelude::*;
use gtk::{CustomSorter, FilterListModel, Frame, Label, NoSelection, SortListModel, StringFilter, StringFilterMatchMode};
use std::cell::Cell;
use std::cmp::Ordering;
use std::rc::Rc;
use crate::gui_frontend::widgets::template_achievements::SamAchievementsPage;

pub fn create_achievements_view(
    // app_id: Rc<Cell<Option<u32>>>,
    // app_unlocked_achievements_count: Rc<Cell<usize>>,
    // _application: &MainApplication,
    // app_achievement_count_value: &Label,
) -> (SamAchievementsPage, ListStore, StringFilter) {
    let app_achievements_model = ListStore::new::<GAchievementObject>();

    let app_achievement_string_filter = StringFilter::builder()
        .expression(GAchievementObject::this_expression("search-text"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let app_achievement_filter_model = FilterListModel::builder()
        .model(&app_achievements_model)
        .filter(&app_achievement_string_filter)
        .build();
    
    let global_achieved_percent_sorter = CustomSorter::new(move |obj1, obj2| {
        let achievement1 = obj1.downcast_ref::<GAchievementObject>().unwrap();
        let achievement2 = obj2.downcast_ref::<GAchievementObject>().unwrap();

        let percent1 = achievement1.global_achieved_percent();
        let percent2 = achievement2.global_achieved_percent();

        percent2
            .partial_cmp(&percent1)
            .unwrap_or(Ordering::Equal)
            .into()
    });
    let app_achievement_sort_model = SortListModel::builder()
        .model(&app_achievement_filter_model)
        .sorter(&global_achieved_percent_sorter)
        .build();

    let app_achievement_selection_model = NoSelection::new(Option::<ListStore>::None);
    app_achievement_selection_model.set_model(Some(&app_achievement_sort_model));

    // let (
    //     achievements_manual_frame,
    // ) = create_achievements_manual_view(
    //     &app_id,
    //     &app_unlocked_achievements_count,
    //     &app_achievement_selection_model,
    //     &app_achievements_model,
    //     app_achievement_count_value,
    // );

    let achievements_page = SamAchievementsPage::default();

    (
        achievements_page,
        app_achievements_model,
        app_achievement_string_filter,
    )
}