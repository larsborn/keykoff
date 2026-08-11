# Changelog

All notable changes to keykoff will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.2.1] - 2026-08-12

### Changed

- The overlay no longer synthesizes keystrokes on every summon. Taking the foreground is now attempted with a plain `SetForegroundWindow` first, and the ALT-tap/`AttachThreadInput` workaround only runs when that is denied (e.g. the Start menu holds the foreground lock). Injecting input on every hotkey press is a behaviour antivirus heuristics score against; this removes it from the common case.
- Replaced the superseded `keybd_event` API with `SendInput`, which also delivers each key sequence atomically so nothing can interleave between keydown and keyup.

## [1.2.0] - 2026-08-09

### Added

- The executable now carries a Windows version resource (product name, description, version, copyright) and an embedded application icon. Metadata-less unsigned binaries score badly with antivirus ML heuristics; this reduces false positives like `Behavior:Win32/Persistence.A!ml`.
- Delete button in the Edit Configuration and Edit Group dialogs, left of Save. Removes the entry (including cascade removal from any groups that reference it) and returns to wherever Save would have gone. Only shown when editing an existing entry, not when creating a new one.

## [1.1.2] - 2026-07-18

### Fixed

- Opening the overlay while another process holds the Windows foreground lock (e.g. the Start menu is open) now actually moves keyboard focus to the input box instead of leaving keystrokes in the previous window. Open shell flyouts (Start menu, Search, Action Center) are dismissed with a simulated ESC, then the overlay takes the foreground via the standard launcher tricks (ALT tap + `AttachThreadInput` + direct `SetForegroundWindow`, verified and retried).
- Cancelling a dialog that was opened from the input overlay (or from a launch error) no longer causes the *next* dialog opened from the config list to wrongly return to Idle instead of the config list after saving.

### Changed

- A corrupt `config.json` is now backed up to `config.json.bak` before defaults are used, instead of being silently overwritten on the next save.
- Failure to register the global hotkey at startup (e.g. the binding is owned by another app) no longer crashes the app; the tray menu remains usable so a different binding can be chosen in the Hotkey tab.

## [1.1.1] - 2026-05-24

### Fixed

- Pressing the hotkey while the input overlay is visible no longer hides it; it now re-focuses the overlay (window + text field) and preserves any typed text. Eliminates flaky "sometimes hides" behavior caused by Windows hotkey auto-repeat. Escape remains the keyboard way to dismiss the overlay.

## [1.1.0] - 2026-04-26

### Added

- Execution groups: bundle multiple programs (or other groups) under a single name. Launching a group launches every reachable program (deduped, depth-first, cycle-protected). Renaming or deleting a referenced entry automatically updates groups that use it. New "+ New Group" button in the Commands tab; group dialog supports type-to-add member autocomplete with self-reference and cycle prevention.
- Right-click (or Ctrl+Enter) on a group in the input overlay now opens the EditGroup dialog (previously a no-op for groups).

### Fixed

- Group dialog: pressing Tab to focus a suggestion and then Enter no longer adds the same member twice.

## [1.0.2] - 2026-04-25

### Fixed

- Config dialog layout: text fields now fill the available window width instead of using fixed sizes, Browse buttons stay right-aligned, and Save/Cancel buttons are right-aligned
- Config dialog is resizable and tracks window size changes properly
- Config dialog has a minimum window size (350x250) to prevent UI overflow

## [1.0.1] - 2026-04-11

### Added

- Autostart tab in settings to register/unregister keykoff in the Windows startup registry key

### Fixed

- Autostart now resolves subst drives and junctions to real filesystem paths so the executable can be found after reboot

## [1.0.0] - 2026-04-03

Initial release. Rewritten from Pascal to Rust.

### Added

- System tray icon with context menu (Configure, Quit)
- Global hotkey (default ALT+F10) to toggle typeahead input overlay
- Typeahead search with numbered results (1-9) for quick selection
- New/edit configuration dialogs with name, caption, executable, parameters, and working directory fields
- Tabbed settings window (Commands, Positioning, Hotkey)
- Configurable overlay position and width
- Configurable hotkey (F1-F12 with ALT/CTRL modifiers)
- Native file/folder picker dialogs for executable and working directory fields
- JSON configuration storage at `%APPDATA%/keykoff/config.json`
- Detached process spawning so launched programs survive keykoff exiting
- GitHub Actions release workflow for automated builds
