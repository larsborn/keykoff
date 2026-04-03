use eframe::egui;

use crate::app::{AppMode, KeykoffApp};

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    let is_edit = matches!(app.mode, AppMode::EditConfig { .. });
    let title = if is_edit {
        "Edit Configuration"
    } else {
        "New Configuration"
    };

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(title);
        ui.add_space(10.0);

        egui::Grid::new("config_form")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Name:");
                let name_resp = ui.add(
                    egui::TextEdit::singleline(&mut app.dialog_name).desired_width(350.0),
                );
                if app.needs_focus {
                    name_resp.request_focus();
                    app.needs_focus = false;
                }
                ui.end_row();

                ui.label("Caption:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.dialog_caption)
                        .desired_width(350.0)
                        .hint_text("Optional description"),
                );
                ui.end_row();

                ui.label("Executable:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dialog_executable)
                            .desired_width(280.0),
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Executables", &["exe", "bat", "cmd", "ps1"])
                            .pick_file()
                        {
                            app.dialog_executable = path.display().to_string();
                        }
                    }
                });
                ui.end_row();

                ui.label("Parameters:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.dialog_parameters).desired_width(350.0),
                );
                ui.end_row();

                ui.label("Working Dir:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.dialog_working_directory)
                            .desired_width(280.0),
                    );
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            app.dialog_working_directory = path.display().to_string();
                        }
                    }
                });
                ui.end_row();
            });

        if let Some(ref error) = app.dialog_error {
            ui.add_space(5.0);
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.add_space(15.0);
        ui.horizontal(|ui| {
            if ui.button("  Save  ").clicked() {
                let return_to_idle = app.dialog_return_to_idle;
                if app.save_dialog_entry() {
                    app.dialog_return_to_idle = false;
                    app.set_mode(if return_to_idle {
                        AppMode::Idle
                    } else {
                        AppMode::ConfigList
                    });
                }
            }
            ui.add_space(10.0);
            if ui.button("Cancel").clicked() {
                app.set_mode(AppMode::Idle);
            }
        });
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.set_mode(AppMode::Idle);
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        let return_to_idle = app.dialog_return_to_idle;
        if app.save_dialog_entry() {
            app.dialog_return_to_idle = false;
            app.set_mode(if return_to_idle {
                AppMode::Idle
            } else {
                AppMode::ConfigList
            });
        }
    }
}
