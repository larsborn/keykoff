use eframe::egui;

use crate::app::{AppMode, KeykoffApp};

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    let is_edit = matches!(app.mode, AppMode::EditConfig { .. });
    let title = if is_edit {
        "Edit Configuration"
    } else {
        "New Configuration"
    };

    let mut save_requested = false;
    let mut cancel_requested = false;
    let mut delete_requested = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(title);
        ui.add_space(10.0);

        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let spacing = ui.spacing().item_spacing.x;
        let row_height = ui.spacing().interact_size.y;
        let row_spacing = 8.0;

        let label_width = ["Name:", "Caption:", "Executable:", "Parameters:", "Working Dir:"]
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

        // Executable
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Executables", &["exe", "bat", "cmd", "ps1"])
                        .pick_file()
                    {
                        app.dialog_executable = path.display().to_string();
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_sized([label_width, row_height], egui::Label::new("Executable:"));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dialog_executable)
                            .desired_width(ui.available_width()),
                    );
                });
            });
        });
        ui.add_space(row_spacing);

        // Parameters
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Parameters:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.dialog_parameters)
                    .desired_width(ui.available_width()),
            );
        });
        ui.add_space(row_spacing);

        // Working Dir
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        app.dialog_working_directory = path.display().to_string();
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_sized([label_width, row_height], egui::Label::new("Working Dir:"));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dialog_working_directory)
                            .desired_width(ui.available_width()),
                    );
                });
            });
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
                    cancel_requested = true;
                }
                ui.add_space(10.0);
                if ui.button("  Save  ").clicked() {
                    save_requested = true;
                }
                if is_edit {
                    ui.add_space(10.0);
                    if ui.button("Delete").clicked() {
                        delete_requested = true;
                    }
                }
            });
        });
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel_requested = true;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        save_requested = true;
    }

    if delete_requested {
        if let AppMode::EditConfig { index } = app.mode {
            app.delete_entry(index);
            app.set_mode(if app.dialog_return_to_idle {
                AppMode::Idle
            } else {
                AppMode::ConfigList
            });
        }
        return;
    }
    if cancel_requested {
        app.set_mode(AppMode::Idle);
        return;
    }
    if save_requested && app.save_dialog_entry() {
        app.set_mode(if app.dialog_return_to_idle {
            AppMode::Idle
        } else {
            AppMode::ConfigList
        });
    }
}
