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
use gtk::Box as GtkBox;

use crate::ui::components::{AdaptiveColumns, page_container};

pub mod custom_eq;
pub mod enhancement;
pub mod eq;
pub mod spatial;
pub mod state;

pub(crate) use custom_eq::CustomEqualizerWidgets;
pub(crate) use enhancement::{BassEngineWidgets, EnhancementWidgets};
pub(crate) use eq::EqualizerWidgets;
pub(crate) use spatial::SpatialAudioWidgets;

#[derive(Clone)]
pub(crate) struct SoundWidgets {
    pub(crate) sound_columns: AdaptiveColumns,
    pub(crate) eq: EqualizerWidgets,
    pub(crate) custom_eq: CustomEqualizerWidgets,
    pub(crate) bass: BassEngineWidgets,
    pub(crate) spatial: SpatialAudioWidgets,
    pub(crate) enhancement: EnhancementWidgets,
}

pub(crate) fn build_sound_page() -> (GtkBox, SoundWidgets) {
    let page = page_container();
    let sound_columns = AdaptiveColumns::new();

    // === LEFT COLUMN: EQ & Custom EQ Card ===
    let custom_eq = custom_eq::build_custom_eq_section();
    let (eq_card, eq) = eq::build_eq_card(&custom_eq);
    sound_columns.left().append(&eq_card);

    // === RIGHT COLUMN: Enhancement & Spatial Audio Cards ===
    let (enhancement_card, bass, enhancement) = enhancement::build_enhancement_card();
    sound_columns.right().append(&enhancement_card);

    let (spatial_card, spatial) = spatial::build_spatial_card();
    sound_columns.right().append(&spatial_card);

    page.append(sound_columns.widget());

    (
        page,
        SoundWidgets {
            sound_columns,
            eq,
            custom_eq,
            bass,
            spatial,
            enhancement,
        },
    )
}
