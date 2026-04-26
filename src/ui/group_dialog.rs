use eframe::egui;

use crate::app::{AppMode, KeykoffApp};
use crate::config::{self, would_cycle, Entry};

fn compute_suggestions(app: &KeykoffApp, editing_name: &str) -> Vec<String> {
    let trimmed = app.dialog_member_input.trim().to_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    app.config
        .entries
        .iter()
        .filter_map(|e| {
            let name = config::entry_name(e);
            if name == editing_name {
                return None; // exclude self
            }
            if app.dialog_members.iter().any(|m| m == name) {
                return None; // already added
            }
            if !name.to_lowercase().contains(&trimmed) {
                return None;
            }
            if would_cycle(&app.config.entries, editing_name, name) {
                return None;
            }
            Some(name.to_string())
        })
        .take(8)
        .collect()
}

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    let is_edit = matches!(app.mode, AppMode::EditGroup { .. });
    let title = if is_edit { "Edit Group" } else { "New Group" };

    // Resolve the "name to compare against for cycle detection / self-exclusion."
    // For edits, this is the name the group had when the dialog opened (so renaming
    // the group inside the dialog doesn't change which entry counts as "self").
    // For new groups, the user-typed name is used.
    let editing_name_owned: String = match app.mode {
        AppMode::EditGroup { index } => match app.config.entries.get(index) {
            Some(Entry::Group(g)) => g.name.clone(),
            _ => String::new(),
        },
        _ => app.dialog_name.clone(),
    };

    let suggestions = compute_suggestions(app, &editing_name_owned);
    if app.dialog_suggestion_index >= suggestions.len() {
        app.dialog_suggestion_index = 0;
    }
    let dropdown_open = !suggestions.is_empty();

    // Capture key events before the UI renders the input box so the dialog
    // gets first crack at Up/Down/Enter/Escape (the TextEdit doesn't consume them).
    let up_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
    let down_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    let mut suggestion_clicked: Option<String> = None;
    let mut remove_idx: Option<usize> = None;
    let mut save_clicked = false;
    let mut cancel_clicked = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(title);
        ui.add_space(10.0);

        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let spacing = ui.spacing().item_spacing.x;
        let row_height = ui.spacing().interact_size.y;
        let row_spacing = 8.0;

        let label_width = ["Name:", "Caption:", "Members:"]
            .iter()
            .map(|t| {
                ui.fonts(|f| {
                    f.layout_no_wrap(t.to_string(), font_id.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                })
            })
            .fold(0.0f32, f32::max)
            + spacing;

        let panel_width = ui.available_width();

        // Name
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Name:"));
            let name_resp = ui.add(
                egui::TextEdit::singleline(&mut app.dialog_name)
                    .desired_width(ui.available_width()),
            );
            if app.needs_focus {
                name_resp.request_focus();
                app.needs_focus = false;
            }
        });
        ui.add_space(row_spacing);

        // Caption
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Caption:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.dialog_caption)
                    .desired_width(ui.available_width())
                    .hint_text("Optional description"),
            );
        });
        ui.add_space(row_spacing);

        // Members input
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Members:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.dialog_member_input)
                    .desired_width(ui.available_width())
                    .hint_text("Type to add..."),
            );
        });

        // Suggestions dropdown
        if dropdown_open {
            ui.indent("group_suggestions", |ui| {
                for (i, suggestion) in suggestions.iter().enumerate() {
                    let selected = i == app.dialog_suggestion_index;
                    let label = egui::SelectableLabel::new(selected, suggestion);
                    if ui.add(label).clicked() {
                        suggestion_clicked = Some(suggestion.clone());
                    }
                }
            });
        }
        ui.add_space(row_spacing);

        // Existing members list with × remove buttons
        ui.indent("group_members_list", |ui| {
            for (i, member) in app.dialog_members.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("- {}", member));
                    if ui.button("x").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
        });

        if let Some(ref error) = app.dialog_error {
            ui.add_space(5.0);
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.add_space(15.0);
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
                ui.add_space(10.0);
                if ui.button("  Save  ").clicked() {
                    save_clicked = true;
                }
            });
        });
    });

    // Apply UI-loop side effects.
    if let Some(i) = remove_idx {
        app.dialog_members.remove(i);
    }
    // If Tab moved focus to a SelectableLabel and Enter activated it, .clicked()
    // returns true AND the keyboard Enter handler below would also fire on the
    // same press — guard against double-add.
    let suggestion_added_via_click = suggestion_clicked.is_some();
    if let Some(name) = suggestion_clicked {
        app.dialog_members.push(name);
        app.dialog_member_input.clear();
        app.dialog_suggestion_index = 0;
    }

    // Keyboard navigation for the suggestions dropdown.
    if dropdown_open {
        if down_pressed && app.dialog_suggestion_index + 1 < suggestions.len() {
            app.dialog_suggestion_index += 1;
        }
        if up_pressed && app.dialog_suggestion_index > 0 {
            app.dialog_suggestion_index -= 1;
        }
    }

    // Enter: if the dropdown is open, add the highlighted suggestion; otherwise save.
    if enter_pressed && !suggestion_added_via_click {
        if dropdown_open {
            if let Some(name) = suggestions.get(app.dialog_suggestion_index).cloned() {
                app.dialog_members.push(name);
                app.dialog_member_input.clear();
                app.dialog_suggestion_index = 0;
            }
        } else {
            save_clicked = true;
        }
    }

    // Escape: if the dropdown is showing, dismiss it (clear input); otherwise cancel.
    if escape_pressed {
        if dropdown_open {
            app.dialog_member_input.clear();
            app.dialog_suggestion_index = 0;
        } else {
            cancel_clicked = true;
        }
    }

    if cancel_clicked {
        app.set_mode(AppMode::Idle);
        return;
    }
    if save_clicked {
        let return_to_idle = app.dialog_return_to_idle;
        if app.save_group_dialog() {
            app.dialog_return_to_idle = false;
            app.set_mode(if return_to_idle { AppMode::Idle } else { AppMode::ConfigList });
        }
    }
}
