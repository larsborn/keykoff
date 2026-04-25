use eframe::egui;
use winreg::enums::*;
use winreg::RegKey;

use crate::app::{AppMode, ConfigTab, KeykoffApp};
use crate::config;
use crate::hotkey;

const AUTOSTART_REG_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_VALUE_NAME: &str = "keykoff";

enum ListAction {
    NewProgram,
    NewGroup,
    Edit(usize),
    Delete(usize),
}

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.config_tab, ConfigTab::Commands, "Commands");
            ui.selectable_value(&mut app.config_tab, ConfigTab::Positioning, "Positioning");
            ui.selectable_value(&mut app.config_tab, ConfigTab::Hotkey, "Hotkey");
            ui.selectable_value(&mut app.config_tab, ConfigTab::Autostart, "Autostart");
        });
        ui.separator();

        match app.config_tab {
            ConfigTab::Commands => show_commands_tab(app, ui),
            ConfigTab::Positioning => show_positioning_tab(app, ui),
            ConfigTab::Hotkey => show_hotkey_tab(app, ui),
            ConfigTab::Autostart => show_autostart_tab(ui),
        }
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.set_mode(AppMode::Idle);
    }
}

fn show_commands_tab(app: &mut KeykoffApp, ui: &mut egui::Ui) {
    let mut action: Option<ListAction> = None;

    ui.add_space(5.0);
    ui.horizontal(|ui| {
        if ui.button("+ New Program").clicked() {
            app.clear_dialog_fields();
            action = Some(ListAction::NewProgram);
        }
        if ui.button("+ New Group").clicked() {
            app.clear_group_dialog_fields();
            action = Some(ListAction::NewGroup);
        }
    });

    ui.add_space(10.0);
    ui.separator();

    // Calculate the widest command name for alignment
    let max_name_width = app
        .config
        .entries
        .iter()
        .map(|e| ui.fonts(|f| f.layout_no_wrap(crate::config::entry_name(e).to_string(), egui::FontId::proportional(14.0), egui::Color32::WHITE).size().x))
        .fold(0.0_f32, f32::max)
        + 8.0; // small padding

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, entry) in app.config.entries.iter().enumerate() {
            let (name, summary): (&str, String) = match entry {
                crate::config::Entry::Program(p) => (p.name.as_str(), format!("-> {}", p.executable)),
                crate::config::Entry::Group(g) => {
                    let summary = if g.members.is_empty() {
                        "-> (empty)".to_string()
                    } else {
                        format!("-> {} member{}", g.members.len(), if g.members.len() == 1 { "" } else { "s" })
                    };
                    (g.name.as_str(), summary)
                }
            };

            ui.horizontal(|ui| {
                // Right-to-left: buttons get space first, text fills the rest
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Delete").clicked() {
                        action = Some(ListAction::Delete(i));
                    }
                    if ui.button("Edit").clicked() {
                        action = Some(ListAction::Edit(i));
                    }
                    // Fill remaining space left-to-right with truncation
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_sized(
                            [max_name_width, ui.spacing().interact_size.y],
                            egui::Label::new(egui::RichText::new(name).strong()),
                        );
                        ui.add(egui::Label::new(summary).truncate());
                    });
                });
            });
            ui.separator();
        }

        if app.config.entries.is_empty() {
            ui.add_space(20.0);
            ui.label("No configurations yet. Click '+ New Program' or '+ New Group' to add one.");
        }
    });

    match action {
        Some(ListAction::NewProgram) => {
            app.clear_dialog_fields();
            app.set_mode(AppMode::NewConfig);
        }
        Some(ListAction::NewGroup) => {
            app.clear_group_dialog_fields();
            app.set_mode(AppMode::NewGroup);
        }
        Some(ListAction::Edit(i)) => match app.config.entries.get(i) {
            Some(crate::config::Entry::Program(_)) => {
                app.populate_program_dialog_from_index(i);
                app.set_mode(AppMode::EditConfig { index: i });
            }
            Some(crate::config::Entry::Group(_)) => {
                app.populate_group_dialog_from_index(i);
                app.set_mode(AppMode::EditGroup { index: i });
            }
            None => {}
        },
        Some(ListAction::Delete(i)) => {
            let deleted_name = app.config.entries.get(i).map(|e| crate::config::entry_name(e).to_string());
            app.config.entries.remove(i);
            if let Some(name) = deleted_name {
                crate::config::cascade_delete(&mut app.config.entries, &name);
            }
            let _ = config::save_config(&app.config);
        }
        None => {}
    }
}

fn show_positioning_tab(app: &mut KeykoffApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.label("Set the screen position where the input overlay appears.");
    ui.add_space(10.0);

    let mut changed = false;

    egui::Grid::new("positioning_grid")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label("X (pixels from left):");
            if ui
                .add(egui::DragValue::new(&mut app.config.overlay_x).range(0.0..=10000.0).speed(1.0))
                .changed()
            {
                changed = true;
            }
            ui.end_row();

            ui.label("Y (pixels from top):");
            if ui
                .add(egui::DragValue::new(&mut app.config.overlay_y).range(0.0..=10000.0).speed(1.0))
                .changed()
            {
                changed = true;
            }
            ui.end_row();

            ui.label("Width:");
            if ui
                .add(egui::DragValue::new(&mut app.config.overlay_width).range(100.0..=2000.0).speed(1.0))
                .changed()
            {
                changed = true;
            }
            ui.end_row();
        });

    if changed {
        let _ = config::save_config(&app.config);
    }
}

fn show_hotkey_tab(app: &mut KeykoffApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.label("Configure the global hotkey that opens the input overlay.");
    ui.add_space(10.0);

    let mut changed = false;

    egui::Grid::new("hotkey_grid")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label("Key:");
            egui::ComboBox::from_id_salt("hotkey_key")
                .selected_text(&app.config.hotkey_key)
                .show_ui(ui, |ui| {
                    for &key in hotkey::AVAILABLE_KEYS {
                        if ui
                            .selectable_value(&mut app.config.hotkey_key, key.to_string(), key)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Modifiers:");
            ui.horizontal(|ui| {
                if ui.checkbox(&mut app.config.hotkey_alt, "ALT").changed() {
                    changed = true;
                }
                if ui.checkbox(&mut app.config.hotkey_ctrl, "CTRL").changed() {
                    changed = true;
                }
            });
            ui.end_row();
        });

    // Ensure at least one modifier is selected
    if !app.config.hotkey_alt && !app.config.hotkey_ctrl {
        app.config.hotkey_alt = true;
        changed = true;
    }

    if changed {
        app.reregister_hotkey();
        let _ = config::save_config(&app.config);
    }

    ui.add_space(15.0);
    let mods = if app.config.hotkey_alt && app.config.hotkey_ctrl {
        "CTRL+ALT"
    } else if app.config.hotkey_ctrl {
        "CTRL"
    } else {
        "ALT"
    };
    ui.label(format!("Current hotkey: {}+{}", mods, app.config.hotkey_key));
}

fn is_autostart_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey(AUTOSTART_REG_KEY) else {
        return false;
    };
    run_key.get_value::<String, _>(AUTOSTART_VALUE_NAME).is_ok()
}

fn set_autostart(enabled: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(AUTOSTART_REG_KEY, KEY_SET_VALUE)
        .map_err(|e| format!("Failed to open registry key: {}", e))?;

    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        // Canonicalize to resolve subst drives and junctions to real paths
        // (subst mappings don't persist across reboots)
        let exe_path = std::fs::canonicalize(&exe_path).unwrap_or(exe_path);
        let exe_str = exe_path.to_string_lossy();
        // Strip \\?\ extended-length prefix added by canonicalize
        let exe_str = exe_str.strip_prefix(r"\\?\").unwrap_or(&exe_str);
        run_key
            .set_value(AUTOSTART_VALUE_NAME, &exe_str)
            .map_err(|e| format!("Failed to set registry value: {}", e))?;
    } else {
        run_key
            .delete_value(AUTOSTART_VALUE_NAME)
            .map_err(|e| format!("Failed to remove registry value: {}", e))?;
    }
    Ok(())
}

fn show_autostart_tab(ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.label("Start keykoff automatically when Windows starts.");
    ui.add_space(10.0);

    let mut enabled = is_autostart_enabled();
    if ui.checkbox(&mut enabled, "Start with Windows").changed() {
        if let Err(e) = set_autostart(enabled) {
            eprintln!("Autostart error: {}", e);
        }
    }
}
