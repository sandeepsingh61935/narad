import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

/** Clicks shorter than this movement (px) count as toggle, not drag. */
const DRAG_THRESHOLD_PX = 6;

/** Demo fallback when daemon is missing: click cycles these states. */
const DEMO_STATES = [
    'idle', 'listening', 'processing', 'inserted', 'copied', 'cancelled', 'error',
];

const ANCHOR_SIZE = 28;
const DEFAULT_MODE = 'plain';
const OUTCOME_IDLE_MS = 1200;

const DAEMON_NAME = 'dev.narad.Daemon';
const DAEMON_PATH = '/dev/narad/Daemon';
const DAEMON_IFACE = 'dev.narad.Daemon';
const HUD_POSITION_REL = ['narad', 'hud-position.json'];

const DaemonIfaceXml = `
<node>
  <interface name="dev.narad.Daemon">
    <method name="Toggle">
      <arg type="s" direction="in" name="mode"/>
    </method>
    <method name="Cancel"/>
    <signal name="StateChanged">
      <arg type="s" name="state"/>
    </signal>
    <signal name="Level">
      <arg type="d" name="rms"/>
    </signal>
    <property name="Mode" type="s" access="read"/>
    <property name="Paused" type="b" access="read"/>
    <property name="HudVisible" type="b" access="readwrite"/>
  </interface>
</node>`;

const DaemonProxy = Gio.DBusProxy.makeProxyWrapper(DaemonIfaceXml);

/**
 * Quiet Precision floating HUD driven by `dev.narad.Daemon` when available.
 * Demo click-cycle remains as fallback if the daemon is missing (FR-DOT-1/2/3, FR-ACT-2/3).
 */
export default class NaradExtension extends Extension {
    enable() {
        this._connections = [];
        this._signalIds = [];
        this._propSignalId = 0;
        this._grab = null;
        this._press = null;
        this._state = 'idle';
        this._mode = DEFAULT_MODE;
        this._daemonConnected = false;
        this._proxy = null;
        this._nameWatch = 0;
        this._escAction = null;
        this._accelHandler = 0;
        this._waveTimer = 0;
        this._elapsedTimer = 0;
        this._elapsedSec = 0;
        this._outcomeTimer = 0;
        this._levelDriven = false;

        this._root = new St.BoxLayout({
            reactive: true,
            can_focus: true,
            track_hover: true,
            vertical: false,
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.CENTER,
        });

        this._column = new St.BoxLayout({
            vertical: true,
            reactive: true,
            y_align: Clutter.ActorAlign.START,
        });
        this._column.spacing = 4;

        this._row = new St.BoxLayout({
            style_class: 'narad-hud',
            vertical: false,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._row.spacing = 10;

        this._anchor = new St.Bin({
            style_class: 'narad-hud-anchor',
            reactive: true,
            track_hover: true,
            width: ANCHOR_SIZE,
            height: ANCHOR_SIZE,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._anchorCore = new St.Widget({
            style_class: 'narad-hud-anchor-core',
            x_expand: false,
            y_expand: false,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._anchorIcon = new St.Icon({
            icon_name: 'audio-input-microphone-symbolic',
            icon_size: 14,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._anchorGlyph = new St.Label({
            text: '✓',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._anchor.set_child(this._anchorCore);

        this._pill = new St.BoxLayout({
            style_class: 'narad-hud-pill',
            vertical: false,
            visible: false,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._pill.spacing = 8;

        this._wave = new St.BoxLayout({
            style_class: 'narad-hud-wave',
            vertical: false,
            y_align: Clutter.ActorAlign.CENTER,
            visible: false,
        });
        this._wave.spacing = 2;
        this._waveBars = [];
        for (let i = 0; i < 4; i++) {
            const bar = new St.Widget({
                style_class: 'narad-hud-wave-bar',
                width: 2,
                height: 4 + i * 2,
            });
            this._waveBars.push(bar);
            this._wave.add_child(bar);
        }

        this._pillIcon = new St.Icon({
            icon_name: 'emblem-synchronizing-symbolic',
            icon_size: 14,
            visible: false,
        });
        this._pillPrimary = new St.Label({
            style_class: 'narad-hud-text',
            text: '',
        });
        this._pillSep1 = new St.Label({
            style_class: 'narad-hud-text-muted',
            text: '·',
            visible: false,
        });
        this._pillSecondary = new St.Label({
            style_class: 'narad-hud-text-accent',
            text: '',
            visible: false,
        });
        this._pillSep2 = new St.Label({
            style_class: 'narad-hud-text-muted',
            text: '·',
            visible: false,
        });
        this._pillTertiary = new St.Label({
            style_class: 'narad-hud-text-muted',
            text: '',
            visible: false,
        });
        this._pillDot = new St.Widget({
            style_class: 'narad-hud-wave-bar',
            width: 6,
            height: 6,
            visible: false,
        });
        this._pillKeycap = new St.Label({
            style_class: 'narad-hud-keycap',
            text: 'Ctrl+V',
            visible: false,
        });

        this._pill.add_child(this._wave);
        this._pill.add_child(this._pillIcon);
        this._pill.add_child(this._pillPrimary);
        this._pill.add_child(this._pillSep1);
        this._pill.add_child(this._pillSecondary);
        this._pill.add_child(this._pillSep2);
        this._pill.add_child(this._pillTertiary);
        this._pill.add_child(this._pillDot);
        this._pill.add_child(this._pillKeycap);

        this._hint = new St.Label({
            style_class: 'narad-hud-hint',
            text: 'Ctrl+Alt+Space',
            visible: false,
        });

        this._row.add_child(this._anchor);
        this._row.add_child(this._pill);
        this._column.add_child(this._row);
        this._column.add_child(this._hint);
        this._root.add_child(this._column);

        this._connect(this._root, 'button-press-event', (_actor, event) => {
            const button = event.get_button();
            if (button === Clutter.BUTTON_SECONDARY) {
                this._onHideClick();
                return Clutter.EVENT_STOP;
            }
            if (button !== Clutter.BUTTON_PRIMARY)
                return Clutter.EVENT_PROPAGATE;
            const [x, y] = event.get_coords();
            this._press = {
                x,
                y,
                hudX: this._root.x,
                hudY: this._root.y,
                moved: false,
            };
            this._grab = global.stage.grab(this._root);
            return Clutter.EVENT_STOP;
        });
        this._connect(this._root, 'motion-event', (_actor, event) => {
            if (!this._press)
                return Clutter.EVENT_PROPAGATE;
            const [x, y] = event.get_coords();
            const dx = x - this._press.x;
            const dy = y - this._press.y;
            if (!this._press.moved) {
                if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX)
                    return Clutter.EVENT_STOP;
                this._press.moved = true;
                this._hint.visible = false;
            }
            this._root.set_position(
                this._press.hudX + dx,
                this._press.hudY + dy,
            );
            return Clutter.EVENT_STOP;
        });
        this._connect(this._root, 'button-release-event', (_actor, event) => {
            if (event.get_button() !== Clutter.BUTTON_PRIMARY)
                return Clutter.EVENT_PROPAGATE;
            if (this._press && !this._press.moved)
                this._onToggleClick();
            else if (this._press && this._press.moved)
                this._saveHudPosition();
            this._releaseGrab();
            this._press = null;
            return Clutter.EVENT_STOP;
        });
        this._connect(this._anchor, 'notify::hover', () => {
            if (this._state === 'idle')
                this._hint.visible = this._anchor.hover;
        });

        // Shell 50 chrome params: trackFullscreen / affectsStruts only
        // (affectsInputRegion was removed; chrome always extends the input region).
        // trackFullscreen: hide HUD over fullscreen apps (DESIGN auto-hide).
        Main.layoutManager.addChrome(this._root, { trackFullscreen: true });
        this._restoreHudPosition();

        this._applyState('idle');
        this._watchDaemon();
    }

    _hudPositionPath() {
        return GLib.build_filenamev(
            [GLib.get_user_config_dir()].concat(HUD_POSITION_REL),
        );
    }

    _restoreHudPosition() {
        const monitor = Main.layoutManager.primaryMonitor;
        const fallbackX = monitor.x + monitor.width - 56;
        const fallbackY = monitor.y + 48;
        let x = fallbackX;
        let y = fallbackY;
        try {
            const path = this._hudPositionPath();
            const [ok, contents] = GLib.file_get_contents(path);
            if (ok) {
                const text = new TextDecoder().decode(contents);
                const data = JSON.parse(text);
                if (typeof data.x === 'number' && typeof data.y === 'number') {
                    x = data.x;
                    y = data.y;
                }
            }
        } catch (_e) {
            /* missing or corrupt — use default corner */
        }
        this._root.set_position(x, y);
    }

    _saveHudPosition() {
        if (!this._root)
            return;
        try {
            const dir = GLib.build_filenamev([GLib.get_user_config_dir(), 'narad']);
            GLib.mkdir_with_parents(dir, 0o755);
            const data = JSON.stringify({
                x: Math.round(this._root.x),
                y: Math.round(this._root.y),
            });
            GLib.file_set_contents(this._hudPositionPath(), data);
        } catch (e) {
            console.warn(`[narad] save HUD position failed: ${e.message}`);
        }
    }

    _connect(actor, signal, handler) {
        const id = actor.connect(signal, handler);
        this._connections.push([actor, id]);
    }

    _watchDaemon() {
        this._nameWatch = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            DAEMON_NAME,
            Gio.BusNameWatcherFlags.NONE,
            () => this._onDaemonAppeared(),
            () => this._onDaemonVanished(),
        );
    }

    _onDaemonAppeared() {
        this._disconnectProxySignals();
        this._proxy = null;
        this._daemonConnected = false;

        try {
            this._proxy = new DaemonProxy(
                Gio.DBus.session,
                DAEMON_NAME,
                DAEMON_PATH,
                (proxy, error) => {
                    if (error) {
                        console.warn(`[narad] D-Bus proxy failed: ${error.message}`);
                        this._proxy = null;
                        this._daemonConnected = false;
                        return;
                    }
                    this._bindProxy(proxy);
                },
            );
        } catch (e) {
            console.warn(`[narad] D-Bus proxy failed: ${e.message}`);
            this._proxy = null;
            this._daemonConnected = false;
        }
    }

    _bindProxy(proxy) {
        this._proxy = proxy;
        this._daemonConnected = true;

        try {
            const mode = proxy.Mode;
            if (mode)
                this._mode = mode;
        } catch (_e) {
            this._mode = DEFAULT_MODE;
        }

        try {
            this._applyHudVisible(proxy.HudVisible !== false);
        } catch (_e) {
            this._applyHudVisible(true);
        }

        this._signalIds.push(
            proxy.connectSignal('StateChanged', (_p, _sender, params) => {
                const state = params[0];
                console.log(`[narad] StateChanged → ${state}`);
                this._applyState(state);
            }),
        );
        this._signalIds.push(
            proxy.connectSignal('Level', (_p, _sender, params) => {
                this._onLevel(params[0]);
            }),
        );
        this._propSignalId = proxy.connect('g-properties-changed', (_p, changed, _inv) => {
            const dict = changed.deep_unpack();
            if (dict.HudVisible !== undefined) {
                const raw = dict.HudVisible;
                const visible = (raw instanceof GLib.Variant) ? raw.unpack() : raw;
                this._applyHudVisible(visible);
            }
            if (dict.Mode !== undefined) {
                const raw = dict.Mode;
                const mode = (raw instanceof GLib.Variant) ? raw.unpack() : raw;
                if (mode)
                    this._mode = mode;
            }
        });

        console.log('[narad] connected to dev.narad.Daemon');
    }

    _onDaemonVanished() {
        this._disconnectProxySignals();
        this._proxy = null;
        this._daemonConnected = false;
        this._unbindEsc();
        console.log('[narad] daemon vanished; demo click-cycle active');
    }

    _disconnectProxySignals() {
        if (!this._proxy)
            return;
        for (const id of this._signalIds) {
            try {
                this._proxy.disconnectSignal(id);
            } catch (_e) {
                /* already gone */
            }
        }
        this._signalIds = [];
        if (this._propSignalId) {
            try {
                this._proxy.disconnect(this._propSignalId);
            } catch (_e) {
                /* already gone */
            }
            this._propSignalId = 0;
        }
    }

    _onToggleClick() {
        if (this._daemonConnected && this._proxy) {
            const mode = this._mode || DEFAULT_MODE;
            console.log(`[narad] Toggle("${mode}")`);
            this._proxy.ToggleRemote(mode, (_result, error) => {
                if (error)
                    console.warn(`[narad] Toggle failed: ${error.message}`);
            });
            return;
        }

        const idx = DEMO_STATES.indexOf(this._state);
        const next = DEMO_STATES[(idx + 1) % DEMO_STATES.length];
        console.log(`[narad] HUD toggle → ${next} (demo; no daemon)`);
        this._applyState(next);
    }

    _onHideClick() {
        console.log('[narad] HUD hide (right-click)');
        if (this._daemonConnected && this._proxy) {
            this._setHudVisibleProperty(false);
        } else {
            this._applyHudVisible(false);
        }
    }

    _setHudVisibleProperty(value) {
        Gio.DBus.session.call(
            DAEMON_NAME,
            DAEMON_PATH,
            'org.freedesktop.DBus.Properties',
            'Set',
            new GLib.Variant('(ssv)', [
                DAEMON_IFACE,
                'HudVisible',
                new GLib.Variant('b', value),
            ]),
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (conn, result) => {
                try {
                    conn.call_finish(result);
                    this._applyHudVisible(value);
                } catch (e) {
                    console.warn(`[narad] set HudVisible failed: ${e.message}`);
                    this._applyHudVisible(value);
                }
            },
        );
    }

    _applyHudVisible(visible) {
        if (this._root)
            this._root.visible = !!visible;
    }

    _onEscCancel() {
        console.log('[narad] Esc → Cancel');
        if (this._daemonConnected && this._proxy) {
            this._proxy.CancelRemote((_result, error) => {
                if (error)
                    console.warn(`[narad] Cancel failed: ${error.message}`);
            });
            return;
        }
        this._applyState('idle');
    }

    _bindEsc() {
        if (this._escAction !== null)
            return;

        const action = global.display.grab_accelerator(
            'Escape',
            Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
        );
        if (action === Meta.KeyBindingAction.NONE) {
            console.warn('[narad] failed to grab Escape');
            return;
        }

        this._escAction = action;
        const name = Meta.external_binding_name_for_action(action);
        Main.wm.allowKeybinding(name, Shell.ActionMode.ALL);

        if (!this._accelHandler) {
            this._accelHandler = global.display.connect(
                'accelerator-activated',
                (_display, activated) => {
                    if (activated === this._escAction && this._state === 'listening')
                        this._onEscCancel();
                },
            );
        }
        console.log('[narad] Esc grab armed (listening)');
    }

    _unbindEsc() {
        if (this._escAction === null)
            return;

        const name = Meta.external_binding_name_for_action(this._escAction);
        Main.wm.allowKeybinding(name, Shell.ActionMode.NONE);
        global.display.ungrab_accelerator(this._escAction);
        this._escAction = null;
        console.log('[narad] Esc grab released');
    }

    _syncEscGrab(state) {
        if (state === 'listening')
            this._bindEsc();
        else
            this._unbindEsc();
    }

    _setAnchorChild(child) {
        if (this._anchor.get_child() === child)
            return;
        this._anchor.set_child(child);
    }

    _applyState(state) {
        const known = [
            'idle', 'listening', 'processing', 'inserted', 'copied', 'cancelled', 'error',
        ];
        if (!known.includes(state)) {
            console.warn(`[narad] unknown state "${state}", treating as idle`);
            state = 'idle';
        }

        this._clearTimers();
        this._levelDriven = false;
        this._state = state;
        this._syncEscGrab(state);

        this._anchor.style_class = 'narad-hud-anchor';
        this._pill.style_class = 'narad-hud-pill';
        this._pill.visible = false;
        this._wave.visible = false;
        this._pillIcon.visible = false;
        this._pillSep1.visible = false;
        this._pillSecondary.visible = false;
        this._pillSep2.visible = false;
        this._pillTertiary.visible = false;
        this._pillDot.visible = false;
        this._pillKeycap.visible = false;
        this._hint.visible = false;
        this._pillPrimary.style_class = 'narad-hud-text';
        this._pillSecondary.style_class = 'narad-hud-text-accent';

        switch (state) {
        case 'idle':
            this._setAnchorChild(this._anchorCore);
            break;

        case 'listening':
            this._anchor.add_style_class_name('narad-hud-anchor-listening');
            this._pill.add_style_class_name('narad-hud-pill-listening');
            this._anchorIcon.icon_name = 'audio-input-microphone-symbolic';
            this._setAnchorChild(this._anchorIcon);
            this._pill.visible = true;
            this._wave.visible = true;
            this._pillPrimary.text = this._mode || DEFAULT_MODE;
            this._pillSep1.visible = true;
            this._pillSecondary.visible = true;
            this._pillSecondary.style_class = 'narad-hud-text-muted';
            this._elapsedSec = 0;
            this._pillSecondary.text = '0:00';
            this._pillDot.visible = true;
            this._startWave();
            this._startElapsed();
            break;

        case 'processing':
            this._anchor.add_style_class_name('narad-hud-anchor-processing');
            this._setAnchorChild(this._anchorCore);
            this._pill.visible = true;
            this._pillIcon.icon_name = 'emblem-synchronizing-symbolic';
            this._pillIcon.visible = true;
            this._pillPrimary.text = 'Transcribing…';
            break;

        case 'inserted':
            this._anchor.add_style_class_name('narad-hud-anchor-copied');
            this._anchorGlyph.text = '✓';
            this._setAnchorChild(this._anchorGlyph);
            this._pill.visible = true;
            this._pill.add_style_class_name('narad-hud-pill-copied');
            this._pillPrimary.style_class = 'narad-hud-text-success';
            this._pillPrimary.text = 'Inserted';
            this._scheduleIdle(OUTCOME_IDLE_MS);
            break;

        case 'copied':
            this._anchor.add_style_class_name('narad-hud-anchor-copied');
            this._anchorGlyph.text = '✓';
            this._setAnchorChild(this._anchorGlyph);
            this._pill.visible = true;
            this._pill.add_style_class_name('narad-hud-pill-copied');
            this._pillPrimary.style_class = 'narad-hud-text-success';
            this._pillPrimary.text = 'Copied to clipboard';
            this._pillSep1.visible = true;
            this._pillSecondary.visible = true;
            this._pillSecondary.style_class = 'narad-hud-text-muted';
            this._pillSecondary.text = '— no focus';
            this._pillKeycap.visible = true;
            this._scheduleIdle(2200);
            break;

        case 'cancelled':
            this._anchor.add_style_class_name('narad-hud-anchor-cancelled');
            this._anchorGlyph.text = '✕';
            this._setAnchorChild(this._anchorGlyph);
            this._pill.visible = true;
            this._pill.add_style_class_name('narad-hud-pill-cancelled');
            this._pillPrimary.style_class = 'narad-hud-text-muted';
            this._pillPrimary.text = 'Cancelled';
            this._scheduleIdle(OUTCOME_IDLE_MS);
            break;

        case 'error':
            this._anchor.add_style_class_name('narad-hud-anchor-error');
            this._anchorIcon.icon_name = 'dialog-warning-symbolic';
            this._setAnchorChild(this._anchorIcon);
            this._pill.visible = true;
            this._pill.add_style_class_name('narad-hud-pill-error');
            this._pillPrimary.style_class = 'narad-hud-text-error';
            this._pillPrimary.text = 'Microphone unavailable';
            this._pillSep1.visible = true;
            this._pillSecondary.visible = true;
            this._pillSecondary.style_class = 'narad-hud-text-muted';
            this._pillSecondary.text = 'Check audio settings';
            this._scheduleIdle(3200);
            break;
        }
    }

    _onLevel(rms) {
        if (this._state !== 'listening')
            return;
        this._levelDriven = true;
        const level = Math.max(0, Math.min(1, Number(rms) || 0));
        for (let i = 0; i < this._waveBars.length; i++) {
            const base = 4 + i * 3;
            this._waveBars[i].height = Math.max(
                2,
                Math.floor(2 + base * level * (0.55 + 0.45 * Math.random())),
            );
        }
    }

    _startWave() {
        this._waveTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 120, () => {
            if (this._state !== 'listening')
                return GLib.SOURCE_REMOVE;
            if (this._levelDriven)
                return GLib.SOURCE_CONTINUE;
            for (let i = 0; i < this._waveBars.length; i++)
                this._waveBars[i].height = 4 + Math.floor(Math.random() * 12);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _startElapsed() {
        this._elapsedTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1000, () => {
            if (this._state !== 'listening')
                return GLib.SOURCE_REMOVE;
            this._elapsedSec += 1;
            const m = Math.floor(this._elapsedSec / 60);
            const s = this._elapsedSec % 60;
            this._pillSecondary.text = `${m}:${String(s).padStart(2, '0')}`;
            return GLib.SOURCE_CONTINUE;
        });
    }

    _scheduleIdle(ms) {
        this._outcomeTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
            this._outcomeTimer = 0;
            if (['inserted', 'copied', 'cancelled', 'error'].includes(this._state))
                this._applyState('idle');
            return GLib.SOURCE_REMOVE;
        });
    }

    _clearTimers() {
        if (this._waveTimer) {
            GLib.source_remove(this._waveTimer);
            this._waveTimer = 0;
        }
        if (this._elapsedTimer) {
            GLib.source_remove(this._elapsedTimer);
            this._elapsedTimer = 0;
        }
        if (this._outcomeTimer) {
            GLib.source_remove(this._outcomeTimer);
            this._outcomeTimer = 0;
        }
    }

    _releaseGrab() {
        if (this._grab) {
            this._grab.dismiss();
            this._grab = null;
        }
    }

    disable() {
        this._clearTimers();
        this._unbindEsc();
        if (this._accelHandler) {
            global.display.disconnect(this._accelHandler);
            this._accelHandler = 0;
        }
        this._releaseGrab();
        this._press = null;
        this._disconnectProxySignals();
        this._proxy = null;
        this._daemonConnected = false;
        if (this._nameWatch) {
            Gio.bus_unwatch_name(this._nameWatch);
            this._nameWatch = 0;
        }
        for (const [actor, id] of this._connections)
            actor.disconnect(id);
        this._connections = [];
        if (this._root) {
            Main.layoutManager.removeChrome(this._root);
            this._root.destroy();
            this._root = null;
        }
        this._anchor = null;
        this._pill = null;
    }
}
