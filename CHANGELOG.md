# Changelog

All notable changes to keykoff will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
