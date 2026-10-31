// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
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
//
// SPDX-License-Identifier: GPL-3.0-only

use gtk::{Box as GtkBox, prelude::*};

use crate::ui::components::page_container;

pub mod features;
pub mod gestures;
pub mod head_motion;
pub mod state;
pub mod utilities;

pub(crate) use features::FeatureTogglesWidgets;
pub(crate) use gestures::GestureSettingsWidgets;
pub(crate) use head_motion::HeadMotionWidgets;
pub(crate) use utilities::UtilityWidgets;

#[derive(Clone)]
pub(crate) struct ControlsWidgets {
    pub(crate) features: FeatureTogglesWidgets,
    pub(crate) head_motion: HeadMotionWidgets,
    pub(crate) gestures: GestureSettingsWidgets,
    pub(crate) utilities: UtilityWidgets,
}

pub(crate) fn build_controls_page() -> (GtkBox, ControlsWidgets) {
    let page = page_container();

    let (features_group, features) = features::build_features_group();
    page.append(&features_group);

    let (gesture_group, call_gesture_group, gestures) = gestures::build_gesture_groups();
    page.append(&gesture_group);
    page.append(&call_gesture_group);

    let (head_group, head_motion) = head_motion::build_head_motion_group();
    page.append(&head_group);

    let (utility_group, utilities) = utilities::build_utilities_group();
    page.append(&utility_group);

    let widgets = ControlsWidgets {
        features,
        head_motion,
        gestures,
        utilities,
    };

    (page, widgets)
}
