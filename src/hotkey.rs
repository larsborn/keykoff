use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

use crate::config::AppConfig;

pub const AVAILABLE_KEYS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];

pub fn code_from_name(name: &str) -> Code {
    match name {
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        _ => Code::F10,
    }
}

pub fn modifiers_from_config(config: &AppConfig) -> Modifiers {
    let mut mods = Modifiers::empty();
    if config.hotkey_alt {
        mods |= Modifiers::ALT;
    }
    if config.hotkey_ctrl {
        mods |= Modifiers::CONTROL;
    }
    mods
}

pub fn hotkey_from_config(config: &AppConfig) -> HotKey {
    let mods = modifiers_from_config(config);
    let code = code_from_name(&config.hotkey_key);
    HotKey::new(if mods.is_empty() { None } else { Some(mods) }, code)
}

pub fn create_hotkey_manager(config: &AppConfig) -> (GlobalHotKeyManager, HotKey) {
    let manager = GlobalHotKeyManager::new().expect("failed to create global hotkey manager");
    let hotkey = hotkey_from_config(config);
    // Registration can fail if another app owns the binding. Don't panic —
    // the release build has no console, so a panic here is an invisible
    // startup death. The tray menu still works, and the user can pick a
    // different binding in the Hotkey tab (which re-registers).
    if let Err(e) = manager.register(hotkey) {
        eprintln!("keykoff: failed to register hotkey: {}", e);
    }
    (manager, hotkey)
}
