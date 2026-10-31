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

pub mod battery;
pub mod header;
pub mod hero;
pub mod noise;
pub mod paired;
pub mod sound;
pub mod state;

use adw::prelude::*;
use gtk::{Box as GtkBox, Orientation};

use crate::ui::components::{AdaptiveColumns, page_container, preferences_content_row};

pub(crate) use battery::BatteryBannerWidgets;
pub(crate) use header::DeviceHeaderWidgets;
pub(crate) use hero::DeviceModelHeroWidgets;
pub(crate) use noise::QuickNoiseWidgets;
pub(crate) use paired::PairedDevicesCardWidgets;
pub(crate) use sound::QuickSoundWidgets;

#[derive(Clone)]
pub(crate) struct HomeWidgets {
    pub(crate) home_columns: AdaptiveColumns,
    pub(crate) header: DeviceHeaderWidgets,
    pub(crate) battery: BatteryBannerWidgets,
    pub(crate) hero: DeviceModelHeroWidgets,
    pub(crate) noise: QuickNoiseWidgets,
    pub(crate) sound: QuickSoundWidgets,
    pub(crate) paired: PairedDevicesCardWidgets,
}

pub(crate) fn build_home_page() -> (GtkBox, HomeWidgets) {
    let page = page_container();

    let (battery_box, battery) = battery::build_battery_banner();
    let (header_row, header) = header::build_device_header(&battery_box);
    let (model_card, hero) = hero::build_model_hero();

    let device_group = adw::PreferencesGroup::builder()
        .title(&crate::t!("device-information"))
        .build();
    device_group.add(&preferences_content_row(&header_row));

    model_card.set_margin_top(8);
    model_card.set_margin_bottom(8);
    model_card.set_margin_start(12);
    model_card.set_margin_end(12);
    device_group.add(&preferences_content_row(&model_card));

    let (mode_group, noise) = noise::build_quick_noise();
    let (sound_card_section, sound) = sound::build_quick_sound();
    let (paired_devices_section, paired) = paired::build_paired_devices_card();

    let columns = AdaptiveColumns::new();
    columns.left().append(&device_group);
    columns.right().append(&mode_group);
    columns.right().append(&sound_card_section);
    columns.right().append(&paired_devices_section);
    page.append(columns.widget());

    {
        let hero = hero.clone();
        columns.container().connect_orientation_notify(move |hc| {
            let is_single = hc.orientation() == Orientation::Vertical;
            hero.update_card_layout(is_single);
        });
    }

    let widgets = HomeWidgets {
        home_columns: columns,
        header,
        battery,
        hero,
        noise,
        sound,
        paired,
    };

    (page, widgets)
}
