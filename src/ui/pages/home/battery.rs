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

use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Orientation};

use crate::{
    protocol::{BatteryPart, BatteryState},
    ui::components::{BatteryIndicator, battery_indicator},
};

#[derive(Clone)]
pub(crate) struct BatteryBannerWidgets {
    left_battery: BatteryIndicator,
    right_battery: BatteryIndicator,
    case_battery: BatteryIndicator,
}

impl BatteryBannerWidgets {
    pub fn render_battery(&self, battery: &BatteryState, case_battery_is_stale: bool) {
        Self::set_battery_indicator(&self.left_battery, battery.left, false);
        Self::set_battery_indicator(&self.right_battery, battery.right, false);
        Self::set_battery_indicator(&self.case_battery, battery.case, case_battery_is_stale);
    }

    pub fn reset(&self) {
        Self::set_battery_indicator(&self.left_battery, None, false);
        Self::set_battery_indicator(&self.right_battery, None, false);
        Self::set_battery_indicator(&self.case_battery, None, false);
    }

    pub fn set_battery_indicator(
        indicator: &BatteryIndicator,
        part: Option<BatteryPart>,
        stale: bool,
    ) {
        if let Some(part) = part {
            indicator
                .percentage
                .set_text(&format!("{}%", part.percentage));
            indicator.charging.set_visible(part.charging && !stale);
            let stale_tooltip = crate::t!("battery-last-read");
            indicator
                .percentage
                .set_tooltip_text(stale.then_some(stale_tooltip.as_str()));
        } else {
            indicator.percentage.set_text("—");
            indicator.percentage.set_tooltip_text(None);
            indicator.charging.set_visible(false);
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl BatteryBannerWidgets {
    pub fn left(&self) -> &BatteryIndicator {
        &self.left_battery
    }

    pub fn right(&self) -> &BatteryIndicator {
        &self.right_battery
    }

    pub fn case(&self) -> &BatteryIndicator {
        &self.case_battery
    }
}

pub(crate) fn build_battery_banner() -> (GtkBox, BatteryBannerWidgets) {
    let batteries = GtkBox::new(Orientation::Horizontal, 12);
    batteries.set_halign(Align::Start);

    let (left_battery_widget, left_battery) = battery_indicator(Some("L"));
    let (right_battery_widget, right_battery) = battery_indicator(Some("R"));
    let (case_battery_widget, case_battery) = battery_indicator(None);

    batteries.append(&left_battery_widget);
    batteries.append(&right_battery_widget);
    batteries.append(&case_battery_widget);

    let widgets = BatteryBannerWidgets {
        left_battery,
        right_battery,
        case_battery,
    };

    (batteries, widgets)
}
