# CLAUDE.md

> **Keep this file up-to-date** as the project evolves — architecture, dependencies, conventions, and build instructions should always reflect the current state.

## Project

keykoff — A Windows quick-launcher that lives in the system tray. Global hotkey (default ALT-F10, configurable) opens a typeahead input overlay to launch programs via user-defined string-to-command mappings.

## Build & Run

```bash
cargo build          # debug build
cargo build --release
cargo run            # runs debug build (console visible)
```

Release builds hide the console window via `#![windows_subsystem = "windows"]`.

## Tech Stack

| Crate | Version | Purpose |
|-------|---------|---------|
| eframe / egui | 0.30 | GUI framework |
| tray-icon | 0.19 | System tray icon + context menu |
| global-hotkey | 0.6 | Configurable hotkey registration |
| serde / serde_json | 1 | Config serialization |
| dirs | 6 | `%APPDATA%` path resolution |
| rfd | 0.15 | Native file/folder picker dialogs |
| winreg | 0.55 | Windows registry access (autostart) |

## Architecture

Single eframe window with mode-based UI switching:

- **Idle** — window parked off-screen (1x1 at -10000,-10000), tray icon + hotkey active
- **Input** — borderless always-on-top overlay at user-configured position, typeahead dropdown with numbered results (1-9)
- **NewConfig / EditConfig** — dialog with name/caption/exe/params/workdir fields; Enter in any field saves; Name field auto-focused on open
- **NewGroup / EditGroup** — dialog for creating or editing an execution group: name, caption, and a type-to-add member list with autocomplete suggestions filtered by substring, with self-reference and cycle prevention.
- **ConfigList** — tabbed settings window (Commands, Positioning, Hotkey, Autostart)

Mode transitions reconfigure window properties (size, position, decorations) via `ViewportCommand`.

### Critical Windows gotchas

- **Never use `Visible(false)`** — it freezes the eframe event loop on Windows. Idle mode parks the window off-screen instead.
- **`set_event_handler` steals from `receiver()`** — after calling `set_event_handler` on `MenuEvent` or `GlobalHotKeyEvent`, the built-in `receiver()` stops receiving events. Events must be forwarded through `mpsc` channels from the handler callbacks.
- **Hotkey fires on both Pressed and Released** — filter for `HotKeyState::Pressed` only to prevent instant toggle-back.
- **`.with_taskbar(false)`** on `ViewportBuilder` hides the app from the Windows taskbar (tray-only).

### Event handling

`tray-icon` and `global-hotkey` `set_event_handler` callbacks forward events through `mpsc` channels and call `ctx.request_repaint()` to wake the UI thread. The `update()` method drains both channels with `while let Ok`. A `request_repaint_after(500ms)` in Idle mode serves as a safety fallback.

### Hotkey behavior by mode

| Current mode | Hotkey action |
|---|---|
| Idle | Switch to Input mode |
| Input | Re-focus overlay (window + text field); never hides. Escape is the only keyboard dismissal. |
| Any other (config dialog, config list) | Bring window to front (`ViewportCommand::Focus`) |

### Navigation flow

- **Input overlay -> Escape** -> return to Idle (only keyboard way to dismiss without launching; the hotkey itself never hides the overlay)
- **Input overlay -> Enter on match** -> launch program or group, return to Idle
- **Input overlay -> Enter on no match** -> open NewConfig dialog (returns to Idle after save, not ConfigList)
- **Input overlay -> Ctrl+Enter or right-click** -> open EditConfig/EditGroup for selected result (returns to Idle after save)
- **Input overlay -> number key 1-9** -> launch corresponding result directly
- **Launch failure** -> open EditConfig with error message (returns to Idle after save)
- **Config dialog -> Save (or Enter in any field)** -> returns to Idle if opened from overlay/launch-error, ConfigList if opened from config list
- **Config dialog -> Cancel or Escape** -> returns to Idle
- **Config list -> opened from tray** -> Escape returns to Idle
- **Commands tab -> "+ New Group" button** -> open NewGroup dialog (returns to Idle after save)
- **Commands tab -> Edit on Group entry** -> open EditGroup dialog (returns to Idle after save)
- **Group launch** -> walks members depth-first, deduplicates programs, launches each via detached process; missing members are silently skipped; cycles are guarded by visited-set

The `dialog_return_to_idle` flag on `KeykoffApp` tracks whether the config dialog should return to Idle (true) or ConfigList (false) after save. The `clear_*_dialog_fields` / `populate_*_dialog_from_index` helpers reset it to false; entry points that want return-to-Idle (overlay, launch-error) set it to true *after* calling them, so a Cancel/Escape can never leak a stale flag into a dialog opened later from the config list.

## Project Structure

```
src/
  main.rs              # Entry point, eframe launch, tray + hotkey wiring, mpsc channels
  app.rs               # AppMode/ConfigTab enums, KeykoffApp struct, eframe::App impl
  config.rs            # RunConfig/AppConfig structs, JSON load/save
  tray.rs              # Tray icon + menu creation (RGBA bytes, no asset files)
  hotkey.rs            # Hotkey registration, key/modifier mapping (F1-F12), re-registration
  launcher.rs          # Process spawning (detached on Windows)
  ui/
    mod.rs             # Re-exports
    input_overlay.rs   # Typeahead search overlay with numbered results (1-9)
    config_dialog.rs   # New/edit configuration form (name, caption, exe, params, workdir)
    config_list.rs     # Tabbed settings: Commands list, Positioning tab, Hotkey tab, Autostart tab
    group_dialog.rs    # New/edit execution group form (name, caption, type-to-add member list)
```

## Data

Configurations are stored as JSON at `%APPDATA%/keykoff/config.json`. If the file exists but fails to parse, it is copied to `config.json.bak` before defaults are used, so a later save can't silently destroy the user's entries.

The `entries` list is polymorphic: each entry has a `kind` field (`program` or `group`) that determines the variant. Backwards compatibility is maintained — entries lacking a `kind` field deserialize as programs.

```json
{
  "entries": [
    {
      "kind": "program",
      "name": "mumble",
      "caption": "Mumble Voice Comms",
      "executable": "C:\\Program Files\\Mumble\\mumble.exe",
      "parameters": "",
      "working_directory": ""
    },
    {
      "kind": "group",
      "name": "comms-suite",
      "caption": "Communications Apps",
      "members": ["mumble", "discord"]
    }
  ],
  "overlay_x": 100.0,
  "overlay_y": 100.0,
  "overlay_width": 400.0,
  "hotkey_key": "F10",
  "hotkey_alt": true,
  "hotkey_ctrl": false
}
```

All fields added after the initial version use `#[serde(default)]` so existing config files load without error.

### Data model

- **`RunConfig`** — `name`, `caption` (optional), `executable`, `parameters`, `working_directory`
- **`RunGroup`** — `name`, `caption` (optional), `members: Vec<String>` (list of entry names)
- **`Entry`** — enum with variants `Program(RunConfig)` and `Group(RunGroup)`, distinguished by `kind` field in JSON
- **`AppConfig`** — `entries: Vec<Entry>`, `overlay_x`, `overlay_y`, `overlay_width`, `hotkey_key`, `hotkey_alt`, `hotkey_ctrl`

## Key implementation details

### Input overlay sizing

- The overlay width is the **maximum** of the configured `overlay_width` (minimum/default) and the measured text width of the widest visible result row. This means long captions expand the overlay rather than being clipped.
- Height is computed dynamically: text input height (24px with results, 22px without) plus `row_height * visible_count`. The `row_height` is 18px — tuned to match egui's `SelectableLabel` actual rendered height with `Body` font and zero item spacing.
- Results are only shown after the user starts typing (empty input = no results).
- Results are capped at 9 (matching the 1-9 keyboard shortcuts).

### Focus management

- `needs_focus` flag is set on mode entry (Input, NewConfig, EditConfig, NewGroup, EditGroup). UI code calls `response.request_focus()` on the first text field and clears the flag.
- `focus_requested` flag is set when the hotkey is pressed while a dialog is open. The `update()` method sends `ViewportCommand::Focus` and clears the flag.

### Hotkey re-registration

The Hotkey tab in ConfigList allows changing the key (F1-F12) and modifiers (ALT, CTRL). On change, `app.reregister_hotkey()` unregisters the old binding and registers the new one via `GlobalHotKeyManager`. At least one modifier is enforced. Initial registration failure at startup (e.g. another app owns the binding) is non-fatal: the app logs and continues so the tray menu stays usable and the user can pick a different binding.

### Process spawning

`launcher.rs` uses `std::process::Command` with Windows `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS` creation flags so launched processes survive keykoff exiting. Parameters are split with `split_whitespace()` (no quoted-arg support).

### Commands list layout

In the Commands tab of ConfigList, each row uses a right-to-left outer layout so Edit/Delete buttons are always visible (allocated first). The remaining space shows the command name (fixed-width column sized to the widest name) and executable path (truncated via `Label::truncate()`).

### Group cascade updates

When a program or group is renamed or deleted, all groups that reference it are automatically updated:
- **Cascade rename** — all member references in other groups are updated to the new name
- **Cascade delete** — the entry is removed from all groups that reference it
- **Cycle prevention** — the group dialog prevents adding a group to itself and prevents adding an entry that would create a cycle (checked during member add via `would_cycle()`)
- **Missing members** — during group launch, missing members are silently skipped; the group still launches all reachable members

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). When preparing a new release, update the changelog based on the git diff and commit messages since the last release. Group changes under Added, Changed, Deprecated, Removed, Fixed, or Security headings as appropriate.

## Conventions

- Rust 2021 edition
- No external build scripts or asset files — tray icon is generated from RGBA bytes at runtime
- Launched processes are fully detached so they outlive keykoff
- Window close button (X) hides to tray; only "Quit" from the tray menu actually exits
- egui's default font lacks many Unicode symbols — use ASCII alternatives (e.g. `->` not arrow characters)
