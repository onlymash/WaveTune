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

import Gio from 'gi://Gio';
import GObject from 'gi://GObject';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {PopupAnimation} from 'resource:///org/gnome/shell/ui/boxpointer.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';

const BUS_NAME = 'io.github.onlymash.WaveTune';
const OBJECT_PATH = '/io/github/onlymash/WaveTune/QuickSettings';

const DEFAULT_ICON_NAME = 'audio-headphones-symbolic';

const QUICK_SETTINGS_IFACE_XML = `
<node>
  <interface name="io.github.onlymash.WaveTune.QuickSettings">
    <property name="Connected" type="b" access="read"/>
    <property name="DeviceName" type="s" access="read"/>
    <property name="ControlsEnabled" type="b" access="read"/>
    <property name="StatusHint" type="s" access="read"/>
    <property name="ActiveGroupIndex" type="i" access="read"/>
    <property name="ActiveModeIndex" type="i" access="read"/>
    <property name="ActiveModeLabel" type="s" access="read"/>
    <property name="ActiveModeIcon" type="s" access="read"/>
    <property name="NoiseControlActive" type="b" access="read"/>
    <property name="PrimaryModes" type="a(iss)" access="read"/>
    <property name="SubModesTitle" type="s" access="read"/>
    <property name="SubModes" type="a(is)" access="read"/>
    <property name="Battery" type="(ibibib)" access="read"/>
    <property name="BatterySummary" type="s" access="read"/>
    <property name="OpenAppLabel" type="s" access="read"/>
    <method name="ToggleNoiseControl"/>
    <method name="SetNoiseMode">
      <arg name="index" type="i" direction="in"/>
    </method>
    <method name="PresentWindow"/>
  </interface>
</node>`;

const WaveTuneProxy = Gio.DBusProxy.makeProxyWrapper(QUICK_SETTINGS_IFACE_XML);
const ShellVersion = parseFloat(Config.PACKAGE_VERSION);

function closeQuickSettingsMenu() {
    const quickSettingsMenu = Main.panel.statusArea.quickSettings;
    if (!quickSettingsMenu?.menu)
        return;

    if (ShellVersion >= 51)
        quickSettingsMenu.menu.close({fadeOnly: true});
    else
        quickSettingsMenu.menu.close(PopupAnimation.FADE);
}

const WaveTuneToggle = GObject.registerClass(
class WaveTuneToggle extends QuickSettings.QuickMenuToggle {
    _init() {
        const defaultIcon = Gio.ThemedIcon.new(DEFAULT_ICON_NAME);
        super._init({
            title: 'WaveTune',
            subtitle: null,
            gicon: defaultIcon,
            toggleMode: false,
        });

        this._activeIconName = DEFAULT_ICON_NAME;
        this._proxy = null;
        this._controlsEnabled = false;
        this._primarySignature = '';
        this._subSignature = '';
        this._primaryItems = new Map();
        this._subItems = new Map();

        this.visible = false;
        this.menu.setHeader(defaultIcon, 'WaveTune', null);

        this._primarySection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._primarySection);

        this._subSeparator = new PopupMenu.PopupSeparatorMenuItem();
        this._subSeparator.visible = false;
        this.menu.addMenuItem(this._subSeparator);

        this._subSection = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._subSection);

        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._openItem = new PopupMenu.PopupMenuItem('Open WaveTune');
        this._openItem.connectObject('activate', () => {
            this._openWaveTune();
        }, this);
        this.menu.addMenuItem(this._openItem);

        this.connectObject('clicked', () => {
            this._onToggleClicked();
        }, this);
    }

    setProxy(proxy) {
        this._proxy = proxy;
        this.syncFromProxy();
    }

    async _onToggleClicked() {
        if (!this._proxy || !this._proxy.g_name_owner)
            return;

        if (!this._controlsEnabled) {
            this.menu.open();
            return;
        }

        try {
            await this._proxy.ToggleNoiseControlAsync();
        } catch (err) {
            console.debug(`[WaveTune] ToggleNoiseControl failed: ${err.message}`);
        }
    }

    async _selectMode(modeIndex) {
        if (!this._proxy || !this._proxy.g_name_owner || !this._controlsEnabled)
            return;

        try {
            await this._proxy.SetNoiseModeAsync(modeIndex);
        } catch (err) {
            console.debug(`[WaveTune] SetNoiseMode(${modeIndex}) failed: ${err.message}`);
        }
    }

    async _openWaveTune() {
        closeQuickSettingsMenu();
        if (!this._proxy || !this._proxy.g_name_owner)
            return;

        try {
            await this._proxy.PresentWindowAsync();
        } catch (err) {
            console.debug(`[WaveTune] PresentWindow failed: ${err.message}`);
        }
    }

    syncFromProxy() {
        const proxy = this._proxy;
        const connected = Boolean(proxy?.g_name_owner && proxy.Connected);
        if (!connected) {
            if (this.menu.isOpen)
                this.menu.close();
            this.visible = false;
            this.checked = false;
            return false;
        }

        const deviceName = proxy.DeviceName || 'WaveTune';
        const controlsEnabled = Boolean(proxy.ControlsEnabled);
        const statusHint = proxy.StatusHint || '';
        const activeGroup = proxy.ActiveGroupIndex ?? -1;
        const activeMode = proxy.ActiveModeIndex ?? -1;
        const activeLabel = proxy.ActiveModeLabel || '';
        const activeIconName = proxy.ActiveModeIcon || DEFAULT_ICON_NAME;
        const noiseActive = Boolean(proxy.NoiseControlActive);
        const primaryModes = proxy.PrimaryModes || [];
        const subModesTitle = proxy.SubModesTitle || '';
        const subModes = proxy.SubModes || [];
        const batterySummary = proxy.BatterySummary || '';
        const openAppLabel = proxy.OpenAppLabel || 'Open WaveTune';

        if (activeIconName !== this._activeIconName) {
            this._activeIconName = activeIconName;
            this.gicon = Gio.ThemedIcon.new(activeIconName);
        }

        this._controlsEnabled = controlsEnabled;
        this.title = deviceName;
        this.checked = noiseActive;

        if (!controlsEnabled && statusHint) {
            this.subtitle = activeLabel ? `${activeLabel} · ${statusHint}` : statusHint;
        } else {
            this.subtitle = activeLabel || null;
        }

        this.menu.setHeader(
            this.gicon,
            deviceName,
            batterySummary || null
        );
        this._openItem.label.text = openAppLabel;
        this._openItem.visible = Main.sessionMode.allowSettings;

        this._syncPrimaryModes(primaryModes, activeGroup, controlsEnabled);
        this._syncSubModes(subModesTitle, subModes, activeMode, controlsEnabled);

        this.visible = true;
        return true;
    }

    _syncPrimaryModes(primaryModes, activeGroup, controlsEnabled) {
        const sig = JSON.stringify(primaryModes);
        if (sig !== this._primarySignature) {
            this._primarySignature = sig;
            this._primarySection.removeAll();
            this._primaryItems.clear();

            for (const [index, label, iconName] of primaryModes) {
                const icon = Gio.ThemedIcon.new(iconName || DEFAULT_ICON_NAME);
                const item = new PopupMenu.PopupImageMenuItem(label, icon);
                item.connectObject('activate', () => {
                    this._selectMode(index);
                }, this);
                this._primaryItems.set(index, item);
                this._primarySection.addMenuItem(item);
            }
        }

        for (const [index, item] of this._primaryItems) {
            item.setOrnament(
                index === activeGroup
                    ? PopupMenu.Ornament.CHECK
                    : PopupMenu.Ornament.NONE
            );
            item.setSensitive(controlsEnabled);
        }
    }

    _syncSubModes(subModesTitle, subModes, activeMode, controlsEnabled) {
        const hasSubModes = subModes.length > 1;
        this._subSeparator.visible = hasSubModes;
        this._subSection.actor.visible = hasSubModes;

        if (!hasSubModes) {
            if (this._subSignature !== '') {
                this._subSignature = '';
                this._subSection.removeAll();
                this._subItems.clear();
            }
            return;
        }

        const sig = JSON.stringify([subModesTitle, subModes]);
        if (sig !== this._subSignature) {
            this._subSignature = sig;
            this._subSection.removeAll();
            this._subItems.clear();

            if (subModesTitle) {
                const titleItem = new PopupMenu.PopupMenuItem(subModesTitle, {
                    reactive: false,
                    can_focus: false,
                });
                titleItem.add_style_class_name('popup-subtitle-menu-item');
                this._subSection.addMenuItem(titleItem);
            }

            for (const [index, label] of subModes) {
                const item = new PopupMenu.PopupMenuItem(label);
                item.connectObject('activate', () => {
                    this._selectMode(index);
                }, this);
                this._subItems.set(index, item);
                this._subSection.addMenuItem(item);
            }
        }

        for (const [index, item] of this._subItems) {
            item.setOrnament(
                index === activeMode
                    ? PopupMenu.Ornament.CHECK
                    : PopupMenu.Ornament.NONE
            );
            item.setSensitive(controlsEnabled);
        }
    }
});

const WaveTuneIndicator = GObject.registerClass(
class WaveTuneIndicator extends QuickSettings.SystemIndicator {
    _init() {
        super._init();

        this._cancellable = new Gio.Cancellable();
        this._proxy = null;

        this._indicator = this._addIndicator();
        this._indicator.gicon = Gio.ThemedIcon.new(DEFAULT_ICON_NAME);
        this._indicator.visible = false;

        this._toggle = new WaveTuneToggle();
        this.quickSettingsItems.push(this._toggle);

        Main.panel.statusArea.quickSettings.addExternalIndicator(this);

        new WaveTuneProxy(
            Gio.DBus.session,
            BUS_NAME,
            OBJECT_PATH,
            (proxy, error) => {
                if (error) {
                    if (!error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED))
                        console.error(`[WaveTune] D-Bus proxy error: ${error.message}`);
                    return;
                }
                this._proxy = proxy;
                this._toggle.setProxy(proxy);
                proxy.connectObject(
                    'g-properties-changed', () => this._sync(),
                    'notify::g-name-owner', () => this._sync(),
                    this
                );
                this._sync();
            },
            this._cancellable,
            Gio.DBusProxyFlags.DO_NOT_AUTO_START_AT_CONSTRUCTION
        );
    }

    _sync() {
        const visible = this._toggle.syncFromProxy();
        this._indicator.gicon = this._toggle.gicon;
        this._indicator.visible = visible;
    }

    destroy() {
        this._cancellable?.cancel();
        this._cancellable = null;

        if (this._proxy) {
            this._proxy.disconnectObject(this);
            this._proxy = null;
        }

        this.quickSettingsItems.forEach(item => item.destroy());
        super.destroy();
    }
});

export default class WaveTuneExtension extends Extension {
    enable() {
        this._indicator = new WaveTuneIndicator();
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}

