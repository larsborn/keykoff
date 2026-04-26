# Changelog

All notable changes to keykoff will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
