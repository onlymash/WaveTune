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

use crate::{compatibility, ui::components::ComboRowExtManual};

#[test]
fn noise_mode_index_selects_the_expected_parent_and_submode() {
    let modes = compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    assert_eq!(compatibility::noise_mode_group_for(modes, 3), Some(0));
    assert_eq!(compatibility::noise_mode_group_for(modes, 1), Some(1));
    assert_eq!(compatibility::noise_mode_group_for(modes, 6), Some(1));
    assert_eq!(compatibility::noise_mode_group_for(modes, 8), Some(2));
    assert_eq!(compatibility::noise_mode_group_for(modes, 2), Some(2));
    assert_eq!(compatibility::noise_mode_group_for(modes, 11), Some(11));
    assert_eq!(compatibility::noise_mode_group_for(modes, 13), Some(11));
    assert_eq!(compatibility::noise_mode_group_for(modes, 10), None);
}

#[test]
fn x4_compatibility_profile_drives_all_noise_option_labels() {
    let modes = compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    assert_eq!(
        compatibility::noise_mode_for(modes, 5).unwrap().kind,
        compatibility::NoiseModeKind::Moderate
    );
    assert_eq!(
        compatibility::noise_mode_for(modes, 7).unwrap().kind,
        compatibility::NoiseModeKind::Smart
    );
    assert_eq!(
        compatibility::noise_mode_for(modes, 11).unwrap().kind,
        compatibility::NoiseModeKind::Adaptive
    );
}

#[test]
fn noise_control_circular_buttons_and_combobox_row() {
    if gtk::init().is_err() {
        return;
    }
    use adw::prelude::*;

    let (_, widgets) = crate::ui::pages::home::build_home_page();

    // Verify first row buttons have circular and noise-mode-button CSS classes and are top-aligned
    for control in widgets.noise.primary_noise_controls() {
        assert!(control.button.has_css_class("circular"));
        assert!(control.button.has_css_class("noise-mode-button"));
        assert!(control.label.has_css_class("noise-mode-label"));
        assert_eq!(control.container.valign(), gtk::Align::Start);
        assert_eq!(control.button.valign(), gtk::Align::Start);
        assert_eq!(control.label.valign(), gtk::Align::Start);
    }

    // Verify second row is a modern ComboRow (initially hidden)
    assert!(!widgets.noise.noise_option_row().is_visible());

    // Verify mode_group card is initially hidden when no device/noise modes are loaded
    assert!(!widgets.noise.mode_group().is_visible());

    // Verify model_firmware label is present in model card, initially hidden and styled
    assert!(!widgets.hero.model_firmware().is_visible());
    assert!(widgets.hero.model_firmware().has_css_class("dim-label"));
    assert!(widgets.hero.model_firmware().has_css_class("caption"));

    let modes = compatibility::profile_for_product_id(0x06ec10)
        .unwrap()
        .noise_modes;
    let wear = crate::protocol::WearState {
        left: Some(crate::protocol::WearPart {
            in_ear: true,
            out_of_case: true,
            raw_flags: 0,
        }),
        right: Some(crate::protocol::WearPart {
            in_ear: true,
            out_of_case: true,
            raw_flags: 0,
        }),
        case_open: Some(true),
    };
    let supported_indices = vec![0, 1, 2, 4, 5, 6, 7, 11, 12, 13, 14];

    // 1. Off mode (index 0): no submodes, noise_option_row remains hidden, mode_group is visible
    crate::ui::home_state::update_noise_controls(
        &widgets,
        &crate::ui::pages::home::noise::NoiseControlsViewState {
            supported_indices: &supported_indices,
            support_note: "",
            capability_enabled: true,
            supports_wear_detection: true,
            wear: &wear,
            current_mode: Some(0),
            modes,
        },
    );
    assert!(widgets.noise.mode_group().is_visible());
    assert!(!widgets.noise.noise_option_row().is_visible());
    let off_ctrl = widgets
        .noise
        .primary_noise_controls()
        .iter()
        .find(|c| c.index == 0)
        .unwrap();
    assert!(off_ctrl.button.has_css_class("suggested-action"));
    assert!(!off_ctrl.label.has_css_class("accent"));

    // 2. Noise Cancellation mode with Smart submode (index 7, parent 1):
    // primary button 1 should be suggested-action, and noise_option_row visible with active index 7
    crate::ui::home_state::update_noise_controls(
        &widgets,
        &crate::ui::pages::home::noise::NoiseControlsViewState {
            supported_indices: &supported_indices,
            support_note: "",
            capability_enabled: true,
            supports_wear_detection: true,
            wear: &wear,
            current_mode: Some(7),
            modes,
        },
    );
    assert!(widgets.noise.noise_option_row().is_visible());
    let anc_ctrl = widgets
        .noise
        .primary_noise_controls()
        .iter()
        .find(|c| c.index == 1)
        .unwrap();
    assert!(anc_ctrl.button.has_css_class("suggested-action"));
    assert!(!anc_ctrl.label.has_css_class("accent"));
    assert!(!off_ctrl.button.has_css_class("suggested-action"));
    assert_eq!(widgets.noise.noise_option_row().selected_id(), Some(7));
    assert_eq!(
        widgets.noise.noise_option_row().title().as_str(),
        &crate::t!("noise-mode-level")
    );

    // 3. Unsupported headset (empty noise modes): mode_group card must be hidden
    crate::ui::home_state::update_noise_controls(
        &widgets,
        &crate::ui::pages::home::noise::NoiseControlsViewState {
            supported_indices: &[],
            support_note: "",
            capability_enabled: false,
            supports_wear_detection: false,
            wear: &wear,
            current_mode: None,
            modes: &[],
        },
    );
    assert!(!widgets.noise.mode_group().is_visible());
    assert!(!widgets.noise.noise_option_row().is_visible());
}

#[test]
#[ignore = "Interactive UI layout and window rendering test"]
fn ui_components_layout_and_behavior() {
    if gtk::init().is_err() {
        return;
    }
    use adw::prelude::*;

    // 1. Button ellipsize test
    let button = gtk::Button::with_label("Noise Cancellation Mode Name That Might Stretch");
    crate::ui::components::configure_button_ellipsize(&button);
    let child = button.child().expect("button has child label");
    let label = child
        .downcast_ref::<gtk::Label>()
        .expect("child is GtkLabel");
    assert_eq!(label.ellipsize(), gtk::pango::EllipsizeMode::End);

    button.set_label("Very Long Noise Cancellation Mode Name That Is Long Enough To Stretch");
    let child = button.child().unwrap();
    let label = child.downcast_ref::<gtk::Label>().unwrap();
    assert_eq!(label.ellipsize(), gtk::pango::EllipsizeMode::None);

    crate::ui::components::configure_button_ellipsize(&button);
    assert_eq!(label.ellipsize(), gtk::pango::EllipsizeMode::End);

    let (min, nat, _, _) = button.measure(gtk::Orientation::Horizontal, -1);
    assert!(min < nat);

    // 2. Sound card preferences group
    let group = adw::PreferencesGroup::builder()
        .title("Sound Quality and Effects")
        .build();

    let string_list = gtk::StringList::new(&["Default"]);
    let eq_row = adw::ComboRow::builder()
        .title("Headset Equalizer")
        .subtitle("Select on-device sound tuning presets that are very long in English")
        .model(&string_list)
        .selected(0)
        .build();
    group.add(&eq_row);

    let switch = gtk::Switch::new();
    let game_mode_row = adw::ActionRow::builder()
        .title("Game Mode")
        .subtitle("Reduce audio transmission latency for better synchronization")
        .activatable_widget(&switch)
        .build();
    game_mode_row.add_suffix(&switch);
    group.add(&game_mode_row);

    let (min, _, _, _) = group.measure(gtk::Orientation::Horizontal, -1);
    assert!(min <= 200);

    // 3. Paired devices list box
    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let row = adw::ActionRow::builder()
        .title("Test Phone")
        .activatable(true)
        .build();
    let icon = gtk::Image::from_icon_name("phone-symbolic");
    row.add_prefix(&icon);
    let chevron = gtk::Image::from_icon_name("go-next-symbolic");
    row.add_suffix(&chevron);
    list_box.append(&row);

    assert!(list_box.first_child().is_some());

    // Verify ButtonRow destructive action preserves the "button" class and matches height
    let connect_btn = adw::ButtonRow::builder().title("Connect").build();
    let delete_btn = adw::ButtonRow::builder().title("Forget Device").build();
    delete_btn.add_css_class("destructive-action");
    assert!(connect_btn.has_css_class("button"));
    assert!(delete_btn.has_css_class("button"));
    assert!(delete_btn.has_css_class("destructive-action"));
    let (c_min, _, _, _) = connect_btn.measure(gtk::Orientation::Vertical, 400);
    let (d_min, _, _, _) = delete_btn.measure(gtk::Orientation::Vertical, 400);
    assert_eq!(
        c_min, d_min,
        "delete_row height ({d_min}) should match connect_row height ({c_min})"
    );

    let (home_page, widgets) = crate::ui::pages::home::build_home_page();
    widgets.paired.set_visible(true);
    widgets.sound.set_visible(true);

    let cols = crate::ui::components::AdaptiveColumns::new();
    assert!(!cols.is_single_column());
    assert_eq!(cols.container().orientation(), gtk::Orientation::Horizontal);
    assert!(cols.container().is_homogeneous());
    cols.set_single_column(true);
    assert!(cols.is_single_column());
    assert_eq!(cols.container().orientation(), gtk::Orientation::Vertical);
    assert!(!cols.container().is_homogeneous());
    cols.set_single_column(false);
    assert!(!cols.is_single_column());
    assert_eq!(cols.container().orientation(), gtk::Orientation::Horizontal);
    assert!(cols.container().is_homogeneous());

    // Verify switch_row
    let (s_row, s_switch) = crate::ui::components::switch_row("Test Switch", Some("Sub"));
    assert_eq!(s_row.title().as_str(), "Test Switch");
    assert_eq!(s_row.subtitle().unwrap().as_str(), "Sub");
    assert!(!s_switch.is_sensitive());

    // Verify combo_row
    let c_row = crate::ui::components::combo_row("Test Combo", None);
    assert_eq!(c_row.title().as_str(), "Test Combo");
    assert!(c_row.subtitle().is_none_or(|s| s.is_empty()));
    assert!(!c_row.is_sensitive());

    // Verify labeled_slider
    let (slider_box, slider) = crate::ui::components::labeled_slider("Min", "Max", 0.0, 10.0, 1.0);
    assert_eq!(slider_box.orientation(), gtk::Orientation::Vertical);
    assert_eq!(slider.adjustment().lower(), 0.0);
    assert_eq!(slider.adjustment().upper(), 10.0);
    assert!(slider.draws_value());
    assert!(slider.has_css_class("labeled-slider"));
    let first_child = slider_box.first_child().expect("slider exists");
    assert_eq!(first_child, slider.upcast::<gtk::Widget>());
    let labels_row = slider_box
        .last_child()
        .and_downcast::<gtk::Box>()
        .expect("labels row exists");
    assert_eq!(labels_row.orientation(), gtk::Orientation::Horizontal);
    let left_child = labels_row.first_child().expect("left child exists");
    assert_eq!(left_child.halign(), gtk::Align::Start);
    let right_child = labels_row.last_child().expect("right child exists");
    assert_eq!(right_child.halign(), gtk::Align::End);

    // Verify flow_button_box
    let fbox = crate::ui::components::flow_button_box(1, 4);
    assert_eq!(fbox.min_children_per_line(), 1);
    assert_eq!(fbox.max_children_per_line(), 4);
    assert_eq!(fbox.selection_mode(), gtk::SelectionMode::None);

    // Verify preferences_content_row
    let dummy_btn = gtk::Button::with_label("Inside Row");
    let pref_row = crate::ui::components::preferences_content_row(&dummy_btn);
    assert!(!pref_row.is_activatable());
    assert!(!pref_row.is_selectable());

    // Verify radio_row
    let (r_row, r_check) = crate::ui::components::radio_row("Test Radio", None);
    assert_eq!(r_row.title().as_str(), "Test Radio");
    assert!(!r_check.is_sensitive());

    // Verify sound_page construction and SoundWidgets layout
    let (_sound_page, sound_widgets) = crate::ui::pages::sound::build_sound_page();
    assert!(!sound_widgets.sound_columns.is_single_column());
    assert_eq!(sound_widgets.eq.preset_count(), 9);
    assert!(sound_widgets.eq.first_child_exists());
    assert_eq!(sound_widgets.spatial.len(), 3);
    assert!(sound_widgets.spatial.first_child_exists());
    assert!(!sound_widgets.custom_eq.is_editor_visible());
    assert_eq!(sound_widgets.bass.slider_range().0, -5.0);
    assert_eq!(sound_widgets.bass.slider_range().1, 5.0);
    assert!(!sound_widgets.bass.is_slider_sensitive());
    assert!(!sound_widgets.custom_eq.is_add_sensitive());
    sound_widgets.sound_columns.set_single_column(true);
    assert!(sound_widgets.sound_columns.is_single_column());

    // Verify Sound page state updates and two-way interaction methods
    // Initial update with Dynaudio presets & all sound features enabled
    crate::ui::sound_state::update_sound_page_features(
        &sound_widgets,
        &crate::ui::sound_state::SoundPageFeaturesViewState {
            supports_equalizer: true,
            supports_custom_equalizer: true,
            supports_spatial_audio: true,
            supports_head_tracking: true,
            supports_bass_engine: true,
            supports_hi_res: true,
            supports_audio_codec: false,
            available_codecs: &[],
            presets: crate::compatibility::DYNAUDIO_EQ_PRESETS,
            custom_equalizers: &[],
        },
    );
    assert!(sound_widgets.eq.is_card_visible());
    assert!(sound_widgets.spatial.is_card_visible());
    assert!(sound_widgets.enhancement.is_card_visible());
    assert!(sound_widgets.enhancement.is_hires_visible());
    assert!(sound_widgets.bass.is_switch_row_visible());
    assert!(sound_widgets.bass.is_slider_row_visible());
    assert!(sound_widgets.bass.is_switch_sensitive());
    assert!(sound_widgets.enhancement.is_hires_sensitive());
    assert!(sound_widgets.custom_eq.is_add_visible());
    assert!(sound_widgets.custom_eq.is_add_sensitive());

    // Exactly 5 Dynaudio preset rows should be visible & sensitive; other 4 hidden
    for idx in 0..9 {
        let is_dynaudio = [0, 7, 2, 1, 3].contains(&idx);
        assert_eq!(sound_widgets.eq.is_preset_visible(idx), is_dynaudio);
        if is_dynaudio {
            assert!(sound_widgets.eq.is_preset_sensitive(idx));
        }
    }

    // All 3 spatial modes (Off, Fixed, Head Tracking) should be visible & sensitive
    for mode in 0..3 {
        assert!(sound_widgets.spatial.is_mode_visible(mode));
        assert!(sound_widgets.spatial.is_mode_sensitive(mode));
    }

    // Select EQ preset 7 (Dynaudio Hi-Res)
    crate::ui::sound_state::update_sound_page_equalizer(&sound_widgets, Some(7));
    assert!(sound_widgets.eq.is_preset_active(7));

    // Select Spatial Audio mode 2 (Head Tracking)
    crate::ui::sound_state::update_sound_page_spatial(&sound_widgets, Some(2));
    assert!(sound_widgets.spatial.is_mode_active(2));

    // Bass engine switch enabled / disabled
    crate::ui::sound_state::update_sound_page_bass_enabled(&sound_widgets, true);
    assert!(sound_widgets.bass.is_switch_active());
    assert!(sound_widgets.bass.is_slider_sensitive());

    crate::ui::sound_state::update_sound_page_bass_enabled(&sound_widgets, false);
    assert!(!sound_widgets.bass.is_switch_active());
    assert!(!sound_widgets.bass.is_slider_sensitive());

    // Bass engine slider range & level
    let level = crate::protocol::BassEngineLevel {
        min: -5,
        max: 5,
        current: 2,
    };
    crate::ui::sound_state::update_sound_page_bass_level(&sound_widgets, level);
    assert_eq!(sound_widgets.bass.slider_value(), 2.0);
    assert_eq!(sound_widgets.bass.slider_range().0, -5.0);
    assert_eq!(sound_widgets.bass.slider_range().1, 5.0);

    // Hi-Res switch state update
    crate::ui::sound_state::update_sound_page_hi_res(&sound_widgets, true);
    assert!(sound_widgets.enhancement.is_hires_active());
    crate::ui::sound_state::update_sound_page_hi_res(&sound_widgets, false);
    assert!(!sound_widgets.enhancement.is_hires_active());

    // Standard presets filter without custom EQ (e.g. Enco X3)
    crate::ui::sound_state::update_sound_page_features(
        &sound_widgets,
        &crate::ui::sound_state::SoundPageFeaturesViewState {
            supports_equalizer: true,
            supports_custom_equalizer: false,
            supports_spatial_audio: true,
            supports_head_tracking: false,
            supports_bass_engine: false,
            supports_hi_res: false,
            supports_audio_codec: false,
            available_codecs: &[],
            presets: crate::compatibility::STANDARD_EQ_PRESETS,
            custom_equalizers: &[],
        },
    );
    assert!(sound_widgets.eq.is_card_visible());
    assert!(sound_widgets.spatial.is_card_visible());
    assert!(!sound_widgets.enhancement.is_card_visible());
    assert!(!sound_widgets.enhancement.is_hires_visible());
    assert!(!sound_widgets.bass.is_switch_row_visible());
    assert!(!sound_widgets.bass.is_slider_row_visible());
    assert!(!sound_widgets.custom_eq.is_add_visible());
    assert!(!sound_widgets.custom_eq.is_add_sensitive());

    for idx in 0..9 {
        if idx == 7 {
            assert!(
                !sound_widgets.eq.is_preset_visible(idx),
                "preset 7 should be hidden for standard models"
            );
        } else {
            assert!(
                sound_widgets.eq.is_preset_visible(idx),
                "preset {idx} should be visible"
            );
        }
    }

    // Spatial mode 2 (head tracking) should be hidden because supports_head_tracking is false
    for mode in 0..3 {
        if mode == 2 {
            assert!(!sound_widgets.spatial.is_mode_visible(mode));
        } else {
            assert!(sound_widgets.spatial.is_mode_visible(mode));
        }
    }

    // Reset sound page
    crate::ui::sound_state::reset_sound_page(&sound_widgets);
    assert!(!sound_widgets.eq.is_card_visible());
    assert!(!sound_widgets.spatial.is_card_visible());
    assert!(!sound_widgets.enhancement.is_card_visible());
    assert!(!sound_widgets.bass.is_switch_sensitive());
    assert!(!sound_widgets.bass.is_switch_active());
    assert!(!sound_widgets.bass.is_slider_sensitive());
    assert_eq!(sound_widgets.bass.slider_value(), 0.0);
    assert!(!sound_widgets.enhancement.is_hires_sensitive());
    assert!(!sound_widgets.enhancement.is_hires_active());
    assert!(!sound_widgets.custom_eq.is_add_sensitive());

    // Enable sound features with custom EQ for custom EQ tests
    crate::ui::sound_state::update_sound_page_features(
        &sound_widgets,
        &crate::ui::sound_state::SoundPageFeaturesViewState {
            supports_equalizer: true,
            supports_custom_equalizer: true,
            supports_spatial_audio: true,
            supports_head_tracking: true,
            supports_bass_engine: true,
            supports_hi_res: true,
            supports_audio_codec: false,
            available_codecs: &[],
            presets: crate::compatibility::DYNAUDIO_EQ_PRESETS,
            custom_equalizers: &[],
        },
    );

    // Custom Equalizers tests: dynamic row insertion, selection, menu, and max cap
    let custom_eq_1 = crate::protocol::CustomEqualizer {
        id: 4,
        name: "自定义1".to_string(),
        min_gain: -6,
        max_gain: 6,
        is_selected: false,
        bands: vec![
            (31, 0),
            (62, 0),
            (125, 0),
            (250, 0),
            (500, 0),
            (1000, 0),
            (2000, 0),
            (4000, 0),
            (8000, 0),
            (16000, 0),
        ],
    };
    let custom_eq_2 = crate::protocol::CustomEqualizer {
        id: 5,
        name: "自定义2".to_string(),
        min_gain: -6,
        max_gain: 6,
        is_selected: true,
        bands: vec![
            (31, 1),
            (62, 2),
            (125, 0),
            (250, 0),
            (500, 0),
            (1000, 0),
            (2000, 0),
            (4000, 0),
            (8000, 0),
            (16000, 0),
        ],
    };
    let custom_eq_3 = crate::protocol::CustomEqualizer {
        id: 6,
        name: "自定义3".to_string(),
        min_gain: -6,
        max_gain: 6,
        is_selected: false,
        bands: vec![
            (31, 0),
            (62, 0),
            (125, 0),
            (250, 0),
            (500, 0),
            (1000, 0),
            (2000, 0),
            (4000, 0),
            (8000, 0),
            (16000, 0),
        ],
    };

    // 1 custom EQ: add_custom_row remains visible
    crate::ui::sound_state::update_sound_page_custom_equalizers(
        &sound_widgets,
        &[custom_eq_1.clone()],
        true,
        |_| {},
        |_, _| {},
        |_, _| {},
    );
    assert_eq!(sound_widgets.custom_eq.preset_count(), 1);
    assert!(sound_widgets.custom_eq.is_add_visible());

    // 3 custom EQs: reaches maximum limit (3), so add_custom_row is hidden
    crate::ui::sound_state::update_sound_page_custom_equalizers(
        &sound_widgets,
        &[
            custom_eq_1.clone(),
            custom_eq_2.clone(),
            custom_eq_3.clone(),
        ],
        true,
        |_| {},
        |_, _| {},
        |_, _| {},
    );
    assert_eq!(sound_widgets.custom_eq.preset_count(), 3);
    assert!(!sound_widgets.custom_eq.is_add_visible());

    // custom_eq_2 was marked is_selected = true, verify its radio is active
    assert!(!sound_widgets.custom_eq.is_preset_active(0));
    assert!(sound_widgets.custom_eq.is_preset_active(1));
    assert!(!sound_widgets.custom_eq.is_preset_active(2));

    // Delete one custom EQ (back to 2): add_custom_row becomes visible again
    crate::ui::sound_state::update_sound_page_custom_equalizers(
        &sound_widgets,
        &[custom_eq_1.clone(), custom_eq_2.clone()],
        true,
        |_| {},
        |_, _| {},
        |_, _| {},
    );
    assert_eq!(sound_widgets.custom_eq.preset_count(), 2);
    assert!(sound_widgets.custom_eq.is_add_visible());

    // Audio codec row tests
    let available_codecs = &[
        crate::protocol::brands::earfun::EarFunAudioCodec::StableConnection.as_u8(),
        crate::protocol::brands::earfun::EarFunAudioCodec::AptX.as_u8(),
        crate::protocol::brands::earfun::EarFunAudioCodec::AptXAdaptive.as_u8(),
        crate::protocol::brands::earfun::EarFunAudioCodec::AptXLossless.as_u8(),
        crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8(),
    ];
    crate::ui::sound_state::update_sound_page_features(
        &sound_widgets,
        &crate::ui::sound_state::SoundPageFeaturesViewState {
            supports_equalizer: true,
            supports_custom_equalizer: true,
            supports_spatial_audio: true,
            supports_head_tracking: true,
            supports_bass_engine: true,
            supports_hi_res: true,
            supports_audio_codec: true,
            available_codecs,
            presets: crate::compatibility::DYNAUDIO_EQ_PRESETS,
            custom_equalizers: &[],
        },
    );
    assert!(sound_widgets.enhancement.is_card_visible());
    assert!(sound_widgets.enhancement.is_codec_visible());
    assert!(sound_widgets.enhancement.is_codec_sensitive());

    // Update active codec to LDAC
    crate::ui::sound_state::update_sound_page_audio_codec(
        &sound_widgets,
        crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8(),
    );
    assert_eq!(
        sound_widgets.enhancement.selected_codec(),
        Some(crate::protocol::brands::earfun::EarFunAudioCodec::Ldac.as_u8())
    );

    // Reset sound page hides codec row and clears selection
    crate::ui::sound_state::reset_sound_page(&sound_widgets);
    assert!(!sound_widgets.enhancement.is_codec_visible());
    assert!(!sound_widgets.enhancement.is_codec_sensitive());
    assert_eq!(sound_widgets.enhancement.selected_codec(), None);

    // Switch to single-column (Vertical)
    widgets.home_columns.set_single_column(true);
    assert!(widgets.home_columns.is_single_column());

    let scroll = gtk::ScrolledWindow::builder()
        .child(&home_page)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .build();

    let window = gtk::Window::builder()
        .default_width(500)
        .default_height(400)
        .child(&scroll)
        .build();
    window.present();

    let ctx = gtk::glib::MainContext::default();
    for _ in 0..10 {
        ctx.iteration(false);
    }

    // Verify home_page can shrink horizontally to fit narrower windows
    let (min_w, _, _, _) = home_page.measure(gtk::Orientation::Horizontal, -1);
    assert!(
        min_w <= 350,
        "home_page minimum width {min_w} should be <= 350 to allow shrinking on narrower displays"
    );

    // Verify single-column model_card height matches the 240px request
    let (_, sc_nat, _, _) = widgets
        .hero
        .model_card()
        .measure(gtk::Orientation::Vertical, 500);
    assert!(
        (sc_nat - 240).abs() <= 20,
        "single-column model_card natural height {sc_nat} should match request 240 (plus padding)"
    );

    // Switch back to two-column (Horizontal) and verify height clamping at 480px
    widgets.home_columns.set_single_column(false);
    for _ in 0..10 {
        ctx.iteration(false);
    }
    let (_, tc_nat, _, _) = widgets
        .hero
        .model_card()
        .measure(gtk::Orientation::Vertical, 800);
    assert!(
        tc_nat <= 500,
        "two-column model_card natural height {tc_nat} must be clamped to at most 480 (plus padding)"
    );

    // Verify device_menu_button and popover setup
    let (btn_min_w, _, _, _) = widgets
        .header
        .menu_button()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(
        btn_min_w <= 60,
        "device_menu_button minimum width {btn_min_w} should be compact"
    );
    assert!(widgets.header.popover().child().is_some());

    // Verify model_card is contained within an adw::PreferencesRow inside device_group
    let model_parent = widgets.hero.model_card().parent();
    assert!(
        model_parent
            .as_ref()
            .and_then(|p| p.downcast_ref::<adw::PreferencesRow>())
            .is_some(),
        "model_card must be wrapped in an adw::PreferencesRow"
    );

    // Verify ComboRow can shrink cleanly with long item text
    widgets.sound.eq_row().set_items(&[(
        0,
        "OPPO Enco X3 Wireless Noise Cancelling Headset (00:11:22:33:44:55)",
    )]);
    widgets.sound.eq_row().set_selected(0);
    let (combo_min_w, _, _, _) = widgets
        .sound
        .eq_row()
        .measure(gtk::Orientation::Horizontal, -1);
    assert!(
        combo_min_w <= 300,
        "eq_row minimum width {combo_min_w} should allow responsive layout"
    );

    // Verify ComboRow includes custom equalizers and preserves selection
    {
        let standard_presets = [
            crate::compatibility::EqualizerPreset {
                index: 0,
                key: "equalizer-preset-classic",
            },
            crate::compatibility::EqualizerPreset {
                index: 1,
                key: "equalizer-preset-bass",
            },
        ];
        let custom_eqs = vec![
            crate::protocol::CustomEqualizer {
                id: 10,
                name: "My Custom EQ".to_string(),
                min_gain: -6,
                max_gain: 6,
                is_selected: false,
                bands: vec![(100, 2), (1000, -1)],
            },
            crate::protocol::CustomEqualizer {
                id: 11,
                name: "Rock Vibes".to_string(),
                min_gain: -6,
                max_gain: 6,
                is_selected: true,
                bands: vec![(100, 4), (1000, 2)],
            },
        ];

        crate::ui::home_state::update_equalizer_items(
            &widgets,
            true,
            &standard_presets,
            &custom_eqs,
            Some(11),
        );

        assert!(widgets.sound.eq_row().is_visible());
        assert!(widgets.sound.eq_row().is_sensitive());
        assert_eq!(
            widgets.sound.eq_row().selected_id(),
            Some(11),
            "Custom EQ 11 should be selected"
        );

        widgets.sound.eq_row().set_selected_id(Some(10));
        assert_eq!(
            widgets.sound.eq_row().selected_id(),
            Some(10),
            "Custom EQ 10 should be selectable"
        );

        widgets.sound.eq_row().set_selected_id(Some(0));
        assert_eq!(
            widgets.sound.eq_row().selected_id(),
            Some(0),
            "Standard preset 0 should be selectable"
        );
    }

    let vadj = scroll.vadjustment();
    assert!(vadj.upper() > vadj.page_size());
    assert!(home_page.height() > 400);

    // 4. Navigation split view collapsed two level navigation test
    let sidebar_toolbar = adw::ToolbarView::new();
    let sidebar_header = adw::HeaderBar::new();
    sidebar_toolbar.add_top_bar(&sidebar_header);
    let sidebar = adw::Sidebar::new();
    let sec = adw::SidebarSection::new();
    sec.append(adw::SidebarItem::builder().title("Home").build());
    sec.append(adw::SidebarItem::builder().title("Sound").build());
    sidebar.append(sec);
    sidebar_toolbar.set_content(Some(&sidebar));
    let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "WaveTune");

    let content_toolbar = adw::ToolbarView::new();
    let content_header = adw::HeaderBar::new();
    content_toolbar.add_top_bar(&content_header);
    let content_label = gtk::Label::new(Some("Content"));
    content_toolbar.set_content(Some(&content_label));
    let content_page = adw::NavigationPage::new(&content_toolbar, "Home");

    let split_view = adw::NavigationSplitView::new();
    split_view.set_sidebar(Some(&sidebar_page));
    split_view.set_content(Some(&content_page));

    // Initially uncollapsed
    split_view.set_collapsed(false);
    assert!(!split_view.is_collapsed());

    // When collapsed below breakpoint
    split_view.set_collapsed(true);
    assert!(split_view.is_collapsed());

    // User is in sidebar (show_content is false)
    split_view.set_show_content(false);
    assert!(!split_view.shows_content());

    // Clicking sidebar item shows content page
    let sv = split_view.clone();
    sidebar.connect_activated(move |_, _| {
        sv.set_show_content(true);
    });
    sidebar.emit_by_name::<()>("activated", &[&1u32]);
    assert!(split_view.shows_content());

    // 5. Stack scrolling with home_page test
    let (hp, w) = crate::ui::pages::home::build_home_page();
    let first = w.sound.sound_card_section().first_child().unwrap();
    let pref_group = first.downcast_ref::<adw::PreferencesGroup>().unwrap();
    assert_eq!(
        pref_group.title().as_str(),
        &crate::t!("sound-quality-and-effects")
    );
    w.paired.set_visible(true);
    w.sound.set_visible(true);
    w.home_columns.set_single_column(true);

    let stack = gtk::Stack::new();
    stack.add_titled(&hp, Some("home"), "Home");
    stack.set_visible_child_name("home");

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&stack)
        .build();

    let win = gtk::Window::builder()
        .default_width(500)
        .default_height(400)
        .child(&scroll)
        .build();
    win.present();
    for _ in 0..10 {
        ctx.iteration(false);
    }
    let vadj = scroll.vadjustment();
    assert!(
        vadj.upper() > vadj.page_size(),
        "upper {} should exceed page_size {} for vertical scrolling",
        vadj.upper(),
        vadj.page_size()
    );

    let parsed = adw::BreakpointCondition::parse("min-width: 800sp and max-width: 1200sp");
    assert!(parsed.is_ok());

    let test_group = adw::PreferencesGroup::new();
    let test_suffix = gtk::Label::new(Some("status"));
    test_group.set_header_suffix(Some(&test_suffix));

    // 6. Test window breakpoints with mutually exclusive conditions
    let test_window = adw::Window::builder()
        .default_width(600)
        .default_height(720)
        .build();

    let sv = adw::NavigationSplitView::new();
    test_window.set_content(Some(&sv));

    let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let medium_bp = adw::Breakpoint::new(
        adw::BreakpointCondition::parse("min-width: 800sp and max-width: 1200sp").unwrap(),
    );
    let bc_m = box_container.clone();
    medium_bp.connect_apply(move |_| {
        bc_m.set_orientation(gtk::Orientation::Vertical);
    });
    let bc_mu = box_container.clone();
    medium_bp.connect_unapply(move |_| {
        bc_mu.set_orientation(gtk::Orientation::Horizontal);
    });
    test_window.add_breakpoint(medium_bp);

    let collapse_bp =
        adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 800sp").unwrap());
    let sv_c = sv.clone();
    let bc_c = box_container.clone();
    collapse_bp.connect_apply(move |_| {
        sv_c.set_collapsed(true);
        bc_c.set_orientation(gtk::Orientation::Vertical);
    });
    let sv_u = sv.clone();
    collapse_bp.connect_unapply(move |_| {
        sv_u.set_collapsed(false);
    });
    test_window.add_breakpoint(collapse_bp);

    test_window.present();
    for _ in 0..15 {
        ctx.iteration(false);
    }

    // At 600 (less than 800sp), collapse_bp should be active
    assert!(
        sv.is_collapsed(),
        "split_view should be collapsed at 600 width"
    );
    assert_eq!(box_container.orientation(), gtk::Orientation::Vertical);

    // Controls page switch state and active checks
    let (_, controls_widgets) = crate::ui::pages::controls::build_controls_page();

    // Initial state: sensitive = false
    assert!(!controls_widgets.features.is_dual_sensitive());
    assert!(!controls_widgets.features.is_in_ear_sensitive());
    assert!(!controls_widgets.features.is_sleep_sensitive());
    assert!(!controls_widgets.head_motion.is_switch_sensitive());
    assert!(!controls_widgets.gestures.is_gesture_combos_sensitive());

    // Enable touch gestures via set_supported
    controls_widgets
        .gestures
        .set_supported(true, crate::compatibility::STANDARD_GESTURE_ROWS);
    assert!(controls_widgets.gestures.is_gesture_combos_sensitive());

    // Update dual device enabled -> true
    crate::ui::controls_state::update_dual_device(&controls_widgets, true);
    assert!(controls_widgets.features.is_dual_state());
    assert!(controls_widgets.features.is_dual_active());
    assert!(controls_widgets.features.is_dual_sensitive());
    assert!(controls_widgets.features.is_dual_row_sensitive());

    // Update in-ear detection enabled -> true
    crate::ui::controls_state::update_in_ear_detection(&controls_widgets, true);
    assert!(controls_widgets.features.is_in_ear_state());
    assert!(controls_widgets.features.is_in_ear_active());
    assert!(controls_widgets.features.is_in_ear_sensitive());

    // Update sleep detection enabled -> true
    crate::ui::controls_state::update_sleep_detection(&controls_widgets, true);
    assert!(controls_widgets.features.is_sleep_state());
    assert!(controls_widgets.features.is_sleep_active());
    assert!(controls_widgets.features.is_sleep_sensitive());

    // Update head switch enabled -> true
    crate::ui::controls_state::update_head_switch(&controls_widgets, true);
    assert!(controls_widgets.head_motion.is_switch_state());
    assert!(controls_widgets.head_motion.is_switch_active());
    assert!(controls_widgets.head_motion.is_switch_sensitive());
    assert!(controls_widgets.head_motion.is_motion_sensitive());

    // Reset controls page
    crate::ui::controls_state::reset_controls_page(&controls_widgets);
    assert!(!controls_widgets.features.is_dual_state());
    assert!(!controls_widgets.features.is_dual_active());
    assert!(!controls_widgets.features.is_dual_sensitive());

    assert!(!controls_widgets.features.is_in_ear_state());
    assert!(!controls_widgets.features.is_in_ear_active());
    assert!(!controls_widgets.features.is_in_ear_sensitive());

    assert!(!controls_widgets.features.is_sleep_state());
    assert!(!controls_widgets.features.is_sleep_active());
    assert!(!controls_widgets.features.is_sleep_sensitive());

    assert!(!controls_widgets.head_motion.is_switch_state());
    assert!(!controls_widgets.head_motion.is_switch_active());
    assert!(!controls_widgets.head_motion.is_switch_sensitive());

    // Verify ComboRow with enable_combo_row_subtitle_wrap wraps long subtitles vertically
    // at dialog width instead of squeezing the selected option label
    let pref_list = gtk::StringList::new(&["头戴式（圆角矩形）", "头戴式（圆形）"]);
    let pref_combo = adw::ComboRow::builder()
        .title("应用图标")
        .subtitle("短说明")
        .model(&pref_list)
        .selected(0)
        .build();
    crate::ui::components::enable_combo_row_subtitle_wrap(&pref_combo);
    let (_, short_h, _, _) = pref_combo.measure(gtk::Orientation::Vertical, 380);
    pref_combo.set_subtitle(
        "应用图标已更新，桌面任务栏可能需要片刻刷新；这是一段较长的副标题说明文字用于验证自动折行",
    );
    let (_, long_h, _, _) = pref_combo.measure(gtk::Orientation::Vertical, 380);
    assert!(
        long_h > short_h,
        "ComboRow height should grow when long subtitle wraps ({long_h} <= {short_h})"
    );
}

#[test]
fn test_format_bass_level() {
    assert_eq!(crate::ui::sound_state::format_bass_level(0), "0");
    assert_eq!(crate::ui::sound_state::format_bass_level(1), "+1");
    assert_eq!(crate::ui::sound_state::format_bass_level(6), "+6");
    assert_eq!(crate::ui::sound_state::format_bass_level(-1), "-1");
    assert_eq!(crate::ui::sound_state::format_bass_level(-5), "-5");
}
