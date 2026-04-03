# Keykoff

Configure and run commands very quickly.

## Motivation

Taking inspiration from the great program [QKLaunch](https://qkl.sourceforge.net/). Apart from actually _having_ an open-source variant of such a tool, my main goal here was to understand how well Opus 4.6 can synthesize Rust-based UI applications.

## How it works

Keykoff lives in your system tray. Press **ALT+F10** (configurable) and a small input overlay appears. Type a few characters to filter your saved commands, then:

- Press **1-9** to launch a result directly
- Press **Enter** to launch the selected result
- Press **Escape** to dismiss
- Press **Ctrl+Enter** or **right-click** a result to edit it

If your input doesn't match any command, pressing Enter opens a dialog to create a new one.

## Getting started

1. Download `keykoff.exe` from the [latest release](../../releases/latest)
2. Run it -- a tray icon appears (no window)
3. Press **ALT+F10** to open the overlay
4. Type a name that doesn't exist yet and press Enter to create your first command
5. Fill in the executable path (and optionally caption, parameters, working directory) and save

### Building from source

```bash
cargo build --release
```

The binary will be at `target/release/keykoff.exe`.

## Configuration

Right-click the tray icon for:

- **Open Input** -- same as pressing the hotkey
- **Edit Configurations** -- manage commands, change overlay position/width, or reconfigure the hotkey
- **Quit** -- exit Keykoff

Settings are stored in `%APPDATA%/keykoff/config.json`.

### Hotkey

The default hotkey is **ALT+F10**. You can change the key (F1-F12) and modifiers (ALT, CTRL, or both) in the Hotkey tab of the configuration dialog.

### Overlay positioning

The overlay position (X, Y) and minimum width are configurable in the Positioning tab.
