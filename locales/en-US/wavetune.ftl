# ==============================================================================
# Common & Status
# ==============================================================================
common-on = On
common-off = Off
common-connect = Connect
common-disconnect = Disconnect
common-connected = Connected
common-disconnected = Disconnected
common-connecting = Connecting…
common-disconnecting = Disconnecting…
common-cancel = Cancel
common-delete = Delete
common-switch = Switch
common-switching = Switching…
common-rename = Rename
common-options = { $param0 } Options
common-status = Status: { $param0 }
common-not-reported = Not reported
common-unconfirmed = Unconfirmed
common-missing = missing
common-current-state = Current State: { $param0 }
common-current-state-unread = Current state: Not read yet
common-current-state-syncing = Current state: Synchronizing

# ==============================================================================
# Navigation
# ==============================================================================
nav-home = Home
nav-sound = Sound
nav-controls = Controls
nav-device = Device
nav-main-menu = Main Menu
nav-quit = Quit

# ==============================================================================
# Preferences & About & Empty State
# ==============================================================================
pref-preferences = Preferences
pref-general = General
pref-language = Language
pref-language-desc = Select display language
pref-language-auto = Follow System
pref-language-restart-hint = Restart the application to apply language changes
pref-app-icon = Application Icon
pref-app-icon-desc = Choose launcher and window icon style
pref-app-icon-default = Headphones (Rounded Square)
pref-app-icon-circle = Headphones (Circle)
pref-app-icon-tws = Earbuds (Rounded Square)
pref-app-icon-tws-circle = Earbuds (Circle)
pref-app-icon-applied-hint = Application icon updated; desktop dock may take a moment to refresh
pref-app-icon-apply-failed = Failed to update user icon: { $param0 }
pref-run-in-background = Keep Running in Background
pref-run-in-background-desc = Hide the main window instead of quitting so headset control and Quick Settings stay active
pref-autostart-background = Start in Background on Login
pref-autostart-background-desc = Automatically launch WaveTune silently in the background when logging in
pref-save-failed = Failed to save setting: { $param0 }
qs-open-app = Open WaveTune
qs-not-worn = Not in ear
qs-syncing = Syncing…
qs-unavailable = Unavailable
tray-no-device = No headset connected
tray-noise-control = Noise Control
tray-quit = Quit WaveTune
tray-background = Running in the background
about-title = About WaveTune
about-description = Unofficial wireless headset control center for Linux (OPPO, OnePlus, realme)
empty-state-no-headset-title = No Headset Connected
empty-state-no-headset-description = Connect your headset in system Bluetooth settings. It will be connected automatically once detected, or choose it from the headset menu in the top-right corner.
empty-state-connecting-title = Connecting to Headset

# ==============================================================================
# Channel & Bluetooth Communication
# ==============================================================================
channel-bluetooth-connection = Bluetooth Connection
channel-desc = Communicates with BlueZ stack using Linux RFCOMM private control channel. Use --probe command line argument for a one-shot read-only probe.
channel-and-diagnostics = Channels &amp; Diagnostics
channel-connecting-to = Connecting to { $param0 } control channel…
channel-connected-syncing = Control channel connected, syncing headset state
channel-disconnected = Control channel disconnected
channel-error = Control channel error: { $param0 }
channel-established = Control channel established
channel-init-complete = Control channel initialization complete
channel-not-connected = Control channel not connected
channel-session-ended = Control channel session ended
channel-status-connected = Status: Control channel connected
channel-status-disconnected = Status: Control channel disconnected
channel-headset-closed = Headset closed RFCOMM control connection
channel-unable-to-open-rfcomm = Unable to open headset RFCOMM control channel: { $param0 }
channel-elnk-rfcomm-not-found = ELNK SPP control service RFCOMM channel not found
channel-sdp-conflicting-rfcomm = SDP returned multiple conflicting ELNK RFCOMM channels
channel-sdp-attr-search-failed = Unable to create SDP attribute search
channel-sdp-service-search-failed = Unable to create SDP service search
channel-sdp-spp-uuid-failed = Unable to create SPP service UUID
channel-start-task-failed = Failed to start Bluetooth background task: { $param0 }
channel-connected-read-only = Control channel connected; { $param0 }
channel-connected-read-only-with-errors = Control channel connected; { $param0 }; partial probe errors: { $param1 }
protocol-command-encode-failed = Unable to encode OPPO command: { $param0 }
protocol-frame-encode-failed = Unable to encode OPPO frame: { $param0 }
protocol-frame-decode-failed = Unable to decode headset frame: { $param0 }
protocol-query-send-failed = Failed to send headset command: { $param0 }
protocol-response-read-failed = Failed to read headset response: { $param0 }
protocol-query-timeout = Timed out waiting for headset response to 0x{ $param0 }
protocol-command-rejected = Headset rejected command 0x{ $param0 }, status { $param1 }

# ==============================================================================
# Device Management
# ==============================================================================
device-title = Device
device-management = Device Management
device-information = Device Information
device-details = Device Details
device-type = Device Type
device-type-phone = Phone
device-type-computer = Computer
device-type-tablet = Tablet
device-type-other = Other Device
device-mac-address = MAC Address
device-name = Device Name: { $param0 }
device-name-placeholder = Device Name: —
device-product-id = Product ID: { $param0 }
device-model-placeholder = Product Model: —
device-refresh-list = Refresh device list
device-refreshing-list = Refreshing device list…
device-start-discovery-failed = Failed to start device discovery: { $param0 }
device-stop-connection-failed = Failed to stop current device connection: { $param0 }
device-read-failed = Failed to read device: { $param0 }
device-status-disconnected = Status: Device disconnected
device-status-discovery-failed = Status: Device discovery failed: { $param0 }
device-status-no-connected = Status: No connected device found
device-status-disconnected-generic = Status: Disconnected
device-no-supported-connected = No supported headset connected
device-none-connected = No connected OnePlus/OPPO/realme headset found
device-found-multiple = Found multiple OnePlus/OPPO/realme headsets; please select a device in the app device list
device-please-select = Please select a discovered device first
device-switching = Switching control device…
device-unknown-model = Unknown Model
device-unknown-model-handshake-only = Unknown models only verify control channel handshake and product identification
device-unrecognized-model-read-only = Unrecognized model; staying in read-only mode
device-unnamed = Unnamed Device
device-numbered = { $param0 } · Device { $param1 }
device-catalog-desc = The catalog describes compatible profiles and does not imply hardware verification; only currently implemented features passing capability checks are enabled. Unrecognized models remain read-only.
device-compatibility = Compatibility
device-handshake-missing-caps = Headset handshake response missing capabilities
device-product-id-incomplete = Headset product ID response incomplete
device-event-list-mismatch = Headset event list length mismatch
device-event-list-incomplete = Headset event list response incomplete
device-no-declared-events = No declared events
device-event-count-mismatch = Event count does not match response length
device-response-missing-status = Response missing success status or event count
device-reading-configuration = Reading headset configuration…
event-list-read = Event list read ({ $param0 })
event-list-invalid = Invalid event capability list: { $param0 }
event-list-query-failed = Event capability query failed: { $param0 }
event-subscribe-rejected = Status notification subscription rejected, status: { $param0 }
event-subscribe-failed = Status notification subscription failed: { $param0 }

# ==============================================================================
# Battery
# ==============================================================================
battery-case = Case
battery-left-ear = Left Ear
battery-left-earbud = Left earbud
battery-right-ear = Right Ear
battery-right-earbud = Right earbud
battery-charging = Charging
battery-last-read = Last read battery level
battery-query-no-data = Battery query returned no battery data
battery-read = Battery read
battery-wear-notifications = Battery/wear status change notifications
battery-response-invalid = Invalid battery response: { $param0 }
battery-query-failed = Battery query failed: { $param0 }

# ==============================================================================
# Wear & Case Lid
# ==============================================================================
wear-detection = Wear Detection
wear-status = Wear Status: { $param0 }
wear-status-placeholder = Wear Status: —
wear-status-not-supported = Wear Status: This model does not support wear detection
wear-status-waiting = Wear status: Waiting for device data
wear-status-no-data = Wear status query returned no status data
wear-in-ear = In ear
wear-out-of-ear = Out of ear
wear-in-case = In case
wear-in-case-status = { $param0 } in case
wear-removed = Removed
wear-removed-in-ear = { $param0 } removed, in-ear
wear-removed-out-of-ear = { $param0 } removed, out-of-ear
wear-status-unknown = { $param0 } status unknown
wear-case-lid-closed = Case Lid: Closed
wear-case-lid-open = Case Lid: Open
wear-case-lid-unknown = Case Lid: Unknown
wear-case-closed = Case closed
wear-case-open = Case open
wear-case-status-unknown = Case status unknown
wear-case-lid-read = Wear/case-lid status read
wear-earbuds-not-in-ear = Earbuds not detected in-ear; please put on earbuds before switching noise mode
wear-auto-pause-resume = Auto Pause &amp; Resume
wear-auto-pause-resume-desc = Automatically pauses playback when either earbud is removed, and resumes when put back on.
wear-fit-test = Ear Tip Fit Test
wear-response-invalid = Invalid wear status response: { $param0 }
wear-query-failed = Wear status query failed: { $param0 }
wear-detection-blocked = Wear detection control is not enabled for this model; command blocked
wear-detection-updated = Wear detection setting updated
wear-detection-update-failed = Failed to update wear detection: { $param0 }

# ==============================================================================
# Firmware
# ==============================================================================
firmware-version-placeholder = Firmware Version: —
firmware-version-single = Firmware Version: { $param0 }
firmware-version-left-right = Firmware Version: Left { $param0 } · Right { $param1 }
firmware-version-full = Firmware Version: Left { $param0 } · Right { $param1 } · Case { $param2 }
firmware-version-read = Firmware version read
firmware-response-invalid = Invalid firmware version response: { $param0 }
firmware-query-failed = Firmware version query failed: { $param0 }

# ==============================================================================
# Noise Mode
# ==============================================================================
noise-mode-title = Noise Control
noise-mode-desc = Noise Mode: { $param0 }
noise-mode-level = Noise Cancellation Level
noise-mode-options = Mode Options
noise-mode-adaptive-preference = Adaptive Preference
noise-mode-off = Off
noise-mode-noise-cancellation = Noise Cancellation
noise-mode-transparency = Transparency
noise-mode-deep = Deep
noise-mode-moderate = Moderate
noise-mode-mild = Mild
noise-mode-smart = Smart
noise-mode-adaptive = Adaptive
noise-mode-standard = Standard
noise-mode-less-noise = Less Noise
noise-mode-more-noise = More Noise
noise-mode-vocal-enhancement = Vocal Enhancement
noise-mode-not-returned = Noise Mode: Device did not return a recognized current mode
noise-mode-blocked = Noise control is not enabled for this model; command blocked
noise-mode-confirmed-and-readback = Noise mode confirmed and read back
noise-mode-readback-unrecognized = Noise mode setting confirmed, but headset readback format was unrecognized
noise-mode-confirmed = Headset confirmed noise mode: { $param0 }
noise-mode-reported = Headset reported noise mode: { $param0 }
noise-mode-waiting-confirm = Waiting for headset to confirm noise mode…
noise-mode-send-failed = Failed to send noise mode command: { $param0 }
noise-mode-switch-failed = Failed to switch noise mode: { $param0 }
noise-mode-reading-and-wear = Reading noise mode and wear state…
noise-mode-read-support-after-connecting = Read noise mode support after connecting headset
noise-mode-profile-declares-available = Model profile declares available noise modes directly; runtime query is supplementary and does not gate features
noise-mode-profile-missing = Model profile does not declare noise modes
noise-mode-transparency-title = Transparency Mode
noise-mode-protocol-mode = Protocol Mode { $param0 }
noise-mode-numbered = Mode { $param0 }
noise-mode-not-in-supported-list = Noise mode index { $param0 } is not in the headset supported mode list
noise-mode-not-enabled-in-profile = Model compatibility profile has not enabled noise mode index { $param0 }
noise-mode-readback-mismatch = Headset confirmed setting, but readback mode index { $param0 } mismatching requested mode; UI keeps previous state

# ==============================================================================
# Dual Device Connection
# ==============================================================================
dual-device-title = Dual-Device Connection
dual-device-desc = Connect 2 Bluetooth devices simultaneously with auto-switching based on use case. Paired devices can be managed on the Home tab.
dual-device-seamless = Seamless Multi-Device Audio
dual-device-turn-on = Turn On
dual-device-not-supported = Current device does not support dual-device management
dual-device-turned-off = Dual-device management is turned off
dual-device-disabled-blocked = Dual-device feature is disabled; operation blocked
dual-device-not-enabled = Headset does not have dual-device connection enabled; cannot manage paired devices
dual-device-please-enable = Please enable dual-device connection on the headset first
dual-device-connect = Connect Device
dual-device-disconnect = Disconnect
dual-device-forget = Forget Device
dual-device-this-host = This Device
dual-device-this-host-label = { $param0 } (This Device)
dual-device-this-host-cannot-disconnect = This device maintains the control connection and cannot be disconnected here
dual-device-cannot-delete-connected = Cannot delete a connected device; please disconnect it first
dual-device-already-in-state = Device is already in target connection state
dual-device-not-in-list = Device is no longer in headset reported list, please refresh and retry
dual-device-status-confirmed = Device connection status confirmed
dual-device-removed = Device removed from headset
dual-device-remove-tooltip = Remove this device from headset paired list
dual-device-waiting-update = Waiting for headset to update device list…
dual-device-no-devices-reported = Headset did not report any paired devices
dual-device-confirmed-operation = Headset confirmed device operation
dual-device-readback-mismatch = Headset confirmed operation, but device list readback state did not match
dual-device-target-missing = Headset confirmed operation, but target device was not found in device list readback
dual-device-deletion-unconfirmed = Headset confirmed deletion, but device list readback still contains the device
dual-device-send-connect-failed = Failed to send device connection command
dual-device-send-disconnect-failed = Failed to send device disconnect command
dual-device-send-delete-failed = Failed to send device deletion command
dual-device-operate-failed = Failed to operate device: { $param0 }
dual-device-read-after-connecting = Read paired devices after connecting headset
dual-device-reading-status = Reading dual-device connection status
dual-device-reading-list = Reading headset paired devices…
dual-device-state-unavailable = Dual-device connection state unavailable: { $param0 }
dual-device-confirm-status-failed = Unable to confirm dual-device connection status, operation disabled: { $param0 }
dual-device-profile-missing = Model profile does not declare dual-device management
dual-device-management-blocked = Paired device management is not enabled for this model; command blocked
dual-device-response-invalid = Invalid dual-device status response: { $param0 }
dual-device-read-list-failed = Unable to read paired device list: { $param0 }
dual-device-readback-list-failed = Failed to read back device list after operation: { $param0 }
dual-device-list-invalid = Invalid paired device list response: { $param0 }
dual-device-event-invalid = Invalid paired device event: { $param0 }
dual-device-this-host-cannot-remove = Cannot remove this device from here
dual-device-toggle-blocked = Dual-device connection toggle is not enabled for this model; command blocked
dual-device-toggle-updated = Dual-device connection setting updated
dual-device-toggle-failed = Failed to update dual-device connection: { $param0 }

# ==============================================================================
# Equalizer
# ==============================================================================
equalizer-title = Headset Equalizer
equalizer-desc = Headset has multiple built-in tuning presets, which can also be quickly switched on the Home tab.
equalizer-select-preset = Select on-device sound tuning presets
equalizer-preset-classic = Classic
equalizer-preset-natural = Natural
equalizer-preset-clear-treble = Clear Treble
equalizer-preset-clear-vocal = Clear Vocal
equalizer-preset-bass-boost = Bass Boost
equalizer-preset-pure-vocal = Pure Vocal
equalizer-preset-dynamic-bass = Dynamic Bass
equalizer-preset-soft = Soft
equalizer-preset-dynamic = Dynamic
equalizer-preset-clear = Clear
equalizer-preset-relaxed = Relaxed
equalizer-preset-dynaudio-classic = Dynaudio Classic
equalizer-preset-dynaudio-hi-res = Dynaudio Hi-Res
equalizer-preset-ultimate-sound = Ultimate Sound
equalizer-preset-hd-clarity = HD Clarity
equalizer-preset-dynaudio-tuning = Dynaudio Featured
equalizer-custom-title = Custom EQ
equalizer-add-custom = Add custom EQ
equalizer-custom-modify = Edit
equalizer-custom-delete = Delete
equalizer-custom-name = Preset Name
equalizer-custom-edit-title = Edit Custom EQ
equalizer-custom-add-title = Add Custom EQ
equalizer-custom-save = Save
equalizer-custom-add = Add
equalizer-custom-delete-confirm = Are you sure you want to delete this custom equalizer preset?
equalizer-custom-max-reached = Maximum number of custom equalizers reached
equalizer-preset-numbered = Preset { $param0 }
equalizer-not-returned = Headset Equalizer Preset: Not returned by device
equalizer-blocked = Equalizer control is not enabled for this model; command blocked
equalizer-applied = Headset applied equalizer preset
equalizer-waiting-confirm = Waiting for headset to confirm equalizer preset…
equalizer-send-failed = Failed to send equalizer command: { $param0 }
equalizer-switch-failed = Failed to switch equalizer preset: { $param0 }
equalizer-custom-default-prefix = Custom
equalizer-custom-default-name = Custom { $param0 }
equalizer-custom-blocked = Custom equalizer control is not enabled for this model; command blocked
equalizer-custom-switching = Switching custom equalizer preset…
equalizer-custom-applied = Headset applied custom equalizer preset
equalizer-custom-switch-failed = Failed to switch custom equalizer: { $param0 }
equalizer-custom-adding = Adding custom equalizer preset…
equalizer-custom-added = Custom equalizer preset added
equalizer-custom-add-failed = Failed to add custom equalizer: { $param0 }
equalizer-custom-saving = Saving custom equalizer preset…
equalizer-custom-saved = Custom equalizer preset updated
equalizer-custom-save-failed = Failed to update custom equalizer: { $param0 }
equalizer-custom-deleting = Deleting custom equalizer preset…
equalizer-custom-deleted = Custom equalizer preset deleted
equalizer-custom-delete-failed = Failed to delete custom equalizer: { $param0 }
equalizer-readback-invalid = Invalid equalizer readback response: { $param0 }
equalizer-readback-mismatch = Headset confirmed setting, but readback preset { $param0 } did not match requested preset { $param1 }; UI keeps previous state


# ==============================================================================
# Spatial Audio
# ==============================================================================
spatial-audio-title = Spatial Audio
spatial-audio-desc = Simulates multi-channel surround sound experience, supporting fixed spatial audio and real-time head tracking.
spatial-audio-effects-desc = Spatial surround sound and head tracking effects
spatial-audio-mode-fixed = Fixed
spatial-audio-mode-head-tracking = Head Tracking
spatial-audio-not-returned = Spatial Audio: Not returned by device
spatial-audio-blocked = Spatial audio control is not enabled for this model; command blocked
spatial-audio-applied = Headset applied spatial audio mode
spatial-audio-waiting-confirm = Waiting for headset to confirm spatial audio mode…
spatial-audio-send-failed = Failed to send spatial audio command: { $param0 }
spatial-audio-switch-failed = Failed to switch spatial audio: { $param0 }
spatial-audio-readback-invalid = Invalid spatial audio readback response: { $param0 }
spatial-audio-readback-mismatch = Headset confirmed setting, but readback spatial audio mode { $param0 } did not match requested mode { $param1 }; UI keeps previous state

# ==============================================================================
# Game Mode
# ==============================================================================
game-mode-title = Game Mode
game-mode-desc = Reduce audio transmission latency for better synchronization
game-mode-card-desc = Reduces audio transmission latency for better synchronization and gaming experience. Can be toggled on the Home tab.
game-mode-ultra-low-latency = Ultra-Low Latency
game-mode-state-enabled = Game mode enabled
game-mode-state-disabled = Game mode disabled
game-mode-not-returned = Game Mode: Not returned by device
game-mode-blocked = Game mode control is not enabled for this model; command blocked
game-mode-waiting-confirm = Waiting for headset to confirm game mode…
game-mode-send-failed = Failed to send game mode command: { $param0 }
game-mode-switch-failed = Failed to switch game mode: { $param0 }

# ==============================================================================
# Bass Engine
# ==============================================================================
bass-engine-title = BassWave dynamic bass
bass-engine-summary = Strengthens low frequencies inside the earbuds.
bass-engine-weak = Weak
bass-engine-strong = Strong
bass-engine-not-returned = BassWave: Not returned by device
bass-engine-send-failed = Failed to send BassWave setting command: { $param0 }
bass-engine-blocked = BassWave control is not enabled for this model; command blocked
bass-engine-waiting-confirm = Waiting for headset to confirm BassWave toggle…
bass-engine-state-enabled = BassWave enabled
bass-engine-state-disabled = BassWave disabled
bass-engine-switch-failed = Failed to switch BassWave: { $param0 }
bass-engine-level-blocked = BassWave level control is not enabled for this model; command blocked
bass-engine-level-waiting-confirm = Waiting for headset to confirm BassWave level…
bass-engine-level-applied = BassWave level set to { $param0 }
bass-engine-level-switch-failed = Failed to switch BassWave level: { $param0 }
bass-engine-readback-invalid = Invalid BassWave readback response: { $param0 }
bass-engine-level-readback-mismatch = Headset confirmed setting, but readback BassWave level { $param0 } did not match requested level { $param1 }; UI keeps previous state

# ==============================================================================
# Hi-Res Mode
# ==============================================================================
hi-res-title = Hi-Res Mode
hi-res-summary = Enables high-definition audio transmission for richer details.
hi-res-not-returned = High-Res Mode: Not returned by device
hi-res-send-failed = Failed to send High-Res Mode setting command: { $param0 }
hi-res-blocked = Hi-Res Mode control is not enabled for this model; command blocked
hi-res-waiting-confirm = Waiting for headset to confirm Hi-Res Mode…
hi-res-state-enabled = Hi-Res Mode enabled
hi-res-state-disabled = Hi-Res Mode disabled
hi-res-switch-failed = Failed to switch Hi-Res Mode: { $param0 }
feature-readback-invalid = Invalid { $param0 } readback response: { $param1 }
feature-readback-mismatch = Headset confirmed setting, but readback { $param0 } state did not match requested state; UI keeps previous state

# ==============================================================================
# Sound Quality & Audio Effects
# ==============================================================================
sound-quality-and-effects = Sound Quality &amp; Effects
sound-spatial-and-enhancement = Spatial Audio and Audio Enhancement
sound-hearing-and-bass = Hearing &amp; Bass Enhancement
sound-audio-enhancement = Audio Enhancement
sound-golden-sound-desc = Supports Golden Sound personalized audio compensation, BassWave dynamic bass boost, and HD audio codec transmission.

# ==============================================================================
# Touch Gestures & Controls
# ==============================================================================
controls-touch-gestures = Touch Gestures
controls-tap-double-tap = Tap / Double Tap
controls-triple-tap = Triple Tap
controls-slide-adjustment = Slide Adjustment
controls-long-press-switch = Long Press Mode Switch
controls-gestures-desc = Supports customizing tap, double tap, triple tap, long press, and stem slide volume adjustments.
controls-not-enabled = This model has not enabled setting controls
controls-feature-settings = Feature Settings
controls-headset-utilities = Headset Utilities
controls-find-earbuds = Find Earbuds
controls-find-and-seal-desc = Play sound to locate earbuds, or check ear tip seal with microphones for optimal noise cancellation.
controls-ear-left = Left Earbud
controls-ear-right = Right Earbud
controls-gesture-single = Single Tap / Squeeze
controls-gesture-double = Double Tap / Squeeze
controls-gesture-triple = Triple Tap / Squeeze
controls-gesture-hold = Squeeze and Hold / Long Press
controls-gesture-slide = Slide
controls-gesture-long-hold = Extra Long Hold
controls-gesture-call-single = Single Squeeze / Tap
controls-gesture-call-double = Double Squeeze / Tap
controls-gesture-call-triple = Triple Squeeze / Tap
controls-gesture-call-hold = Squeeze and Hold
controls-gesture-call-other = Other Call Gesture
controls-call-gestures-title = When on a Call
controls-noise-cycle-title = Noise modes cycled by holding
controls-noise-cycle-note = Select at least 2 modes to cycle through on hold
controls-noise-cycle-anc = Noise Cancellation
controls-noise-cycle-transparency = Transparency
controls-noise-cycle-off = Off
controls-function-none = None
controls-function-play-pause = Play / Pause
controls-function-voice-assistant = Voice Assistant
controls-function-previous = Previous Track
controls-function-next = Next Track
controls-function-volume-control = Volume Control
controls-function-noise-control = Noise Control
controls-function-switch-track = Switch Track
controls-function-volume-up = Volume Up
controls-function-volume-down = Volume Down
controls-function-switch-devices = Switch Devices
controls-function-game-mode = Game Mode
controls-function-decline-end-call = Decline / End Call
controls-function-answer-end-call = Answer / End Call
controls-function-unknown = Unknown Function
controls-head-gestures-title = Head Motion Control
controls-head-gestures-desc = Answer or decline incoming calls by nodding or shaking your head.
controls-head-nod-answers = Nod to answer, shake to decline
controls-head-shake-answers = Shake to answer, nod to decline
controls-find-play = Play Sound
controls-find-stop = Stop Playing
controls-find-playing-note = Ringing… Sound stops after 50 seconds or when placed into case.
controls-find-in-ear-title = Earbud In-Ear Warning
controls-find-in-ear-message = Earbuds are currently worn in-ear. Playing loud sound may damage hearing. Are you sure?
controls-find-in-case = Earbuds are in the case, unable to play sound
controls-fit-test-title = Earbud Fit Test
controls-fit-test-desc = Wear both earbuds and stay quiet in a quiet environment to test seal quality.
controls-fit-test-start = Start Test
controls-fit-test-running = Testing earbud fit, please keep quiet…
controls-fit-test-good = Good seal
controls-fit-test-average = Average fit
controls-fit-test-poor = Poor fit, please adjust ear tip
controls-fit-test-failed = Test failed, please retry
controls-fit-test-wear-both = Please wear both earbuds first
controls-prompt-volume-title = Alert Sound Volume
controls-prompt-volume-desc = Adjust the volume of mode switching and status prompt tones.
controls-sleep-detection-title = Sleep Detection
controls-sleep-detection-desc = Automatically pauses playback when falling asleep to save power and protect hearing.
controls-preferred-device-title = Preferred Connection Device
controls-preferred-device-desc = Earbuds prioritize connecting to this device when in dual connection range.
controls-preferred-device-auto = Auto Select
controls-gesture-blocked = Gesture customization is not enabled for this model; command blocked
controls-gesture-updating = Updating headset gesture…
controls-gesture-updated = Gesture setting updated
controls-gesture-update-failed = Failed to update gesture: { $param0 }
controls-noise-cycle-blocked = Noise mode cycle control is not enabled for this model; command blocked
controls-noise-cycle-updating = Updating noise mode cycle…
controls-noise-cycle-updated = Noise mode cycle updated
controls-noise-cycle-update-failed = Failed to update noise mode cycle: { $param0 }
controls-find-blocked = Find Earbuds is not enabled for this model; command blocked
controls-find-ringing = Headset is playing alert sound
controls-find-stopped = Stopped playing alert sound
controls-find-failed = Find Earbuds command failed: { $param0 }
controls-fit-test-blocked = Earbud Fit Test is not enabled for this model; command blocked
controls-fit-test-start-failed = Failed to start fit test: { $param0 }
controls-prompt-volume-blocked = Alert sound volume control is not enabled for this model; command blocked
controls-prompt-volume-updated = Alert sound volume updated
controls-prompt-volume-update-failed = Failed to update alert sound volume: { $param0 }
controls-head-motion-blocked = Head motion control is not enabled for this model; command blocked
controls-head-motion-updated = Head motion control updated
controls-head-motion-update-failed = Failed to update head motion control: { $param0 }
controls-sleep-detection-blocked = Sleep detection control is not enabled for this model; command blocked
controls-sleep-detection-updated = Sleep detection setting updated
controls-sleep-detection-update-failed = Failed to update sleep detection: { $param0 }
controls-preferred-device-blocked = Preferred device setting is not enabled for this model; command blocked
controls-preferred-device-updated = Preferred connection device updated
controls-preferred-device-update-failed = Failed to update preferred device: { $param0 }


# ==============================================================================
# Probe Diagnostics
# ==============================================================================
probe-device = Device: { $param0 }
probe-model-profile = Model Compatibility Profile: { $param0 }
probe-product-id = Product ID: { $param0 }
probe-color-id = Color ID: { $param0 }
probe-firmware = Firmware: Left { $param0 }, Right { $param1 }, Case { $param2 }
probe-noise-mode-index = Noise mode index: { $param0 }
probe-noise-mode-support = Noise mode support info: { $param0 }
probe-headset-eq = Headset Equalizer Preset: { $param0 }
probe-spatial-audio = Spatial Audio: { $param0 }
probe-game-mode = Game Mode: { $param0 }
probe-dual-device = Dual-Device Connection: { $param0 }
probe-in-ear-detection = In-Ear Detection: { $param0 }
probe-paired-devices = Paired Devices: { $param0 }
probe-paired-devices-failed = Paired devices query failed: { $param0 }
probe-bass-engine = BassWave: { $param0 } (level { $param1 }, range { $param2 }..{ $param3 })
probe-bass-engine-simple = BassWave: { $param0 }
probe-hi-res = High-Res Mode: { $param0 }
probe-not-reported = { $param0 }: Not reported by device
probe-detected-read-only = Detected read-only features: { $param0 }
probe-no-queries-confirmed = No available status queries confirmed
probe-note = Probe note: { $param0 }
probe-custom-eq-list = Custom equalizers: { $param0 }
probe-custom-eq-none = Custom equalizers: None
probe-model-no-queries-confirmed = { $param0 } could not confirm available read-only status queries
probe-model-detected-read-only = { $param0 } detected read-only features: { $param1 }
probe-read-only-controls-note = { $param0 }; model profile does not declare setting controls
