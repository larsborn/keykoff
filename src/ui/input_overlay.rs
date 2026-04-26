use eframe::egui;

use crate::app::{AppMode, KeykoffApp};

const NUMBER_KEYS: [egui::Key; 9] = [
    egui::Key::Num1,
    egui::Key::Num2,
    egui::Key::Num3,
    egui::Key::Num4,
    egui::Key::Num5,
    egui::Key::Num6,
    egui::Key::Num7,
    egui::Key::Num8,
    egui::Key::Num9,
];

fn open_edit(app: &mut KeykoffApp, config_idx: usize) {
    match app.config.entries.get(config_idx) {
        Some(crate::config::Entry::Program(_)) => {
            app.populate_program_dialog_from_index(config_idx);
            app.dialog_return_to_idle = true;
            app.set_mode(AppMode::EditConfig { index: config_idx });
        }
        Some(crate::config::Entry::Group(_)) => {
            app.populate_group_dialog_from_index(config_idx);
            app.dialog_return_to_idle = true;
            app.set_mode(AppMode::EditGroup { index: config_idx });
        }
        None => {}
    }
}

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    app.update_filtered_results();

    // Collect key events before UI rendering
    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let ctrl_enter = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter));
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let down_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let up_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));

    // Check number keys 1-9
    let mut number_pressed: Option<usize> = None;
    for (i, key) in NUMBER_KEYS.iter().enumerate() {
        if ctx.input(|inp| inp.key_pressed(*key)) {
            number_pressed = Some(i);
            break;
        }
    }

    let mut launch_index: Option<usize> = None;
    let mut edit_index: Option<usize> = None;

    let visible_count = app.filtered_indices.len().min(9);

    // Measure widest result row to determine window width
    let min_width = app.config.overlay_width;
    let content_width = ctx.fonts(|f| {
        let font = egui::FontId::proportional(14.0);
        let mut max_w = 0.0_f32;
        for (display_idx, &config_idx) in app.filtered_indices.iter().enumerate().take(9) {
            let (name, caption): (&str, &str) = match &app.config.entries[config_idx] {
                crate::config::Entry::Program(p) => (p.name.as_str(), p.caption.as_str()),
                crate::config::Entry::Group(g) => (g.name.as_str(), g.caption.as_str()),
            };
            let text = if caption.is_empty() {
                format!("{}  {}", display_idx + 1, name)
            } else {
                format!("{}  {} - {}", display_idx + 1, name, caption)
            };
            let w = f.layout_no_wrap(text, font.clone(), egui::Color32::WHITE).size().x;
            max_w = max_w.max(w);
        }
        max_w + 16.0 // padding for SelectableLabel margins
    });
    let width = min_width.max(content_width);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            let response = ui.add(
                egui::TextEdit::singleline(&mut app.search_text)
                    .desired_width(f32::INFINITY)
                    .hint_text("Type to filter...")
                    .font(egui::TextStyle::Body)
                    .margin(egui::Margin::symmetric(4.0, 4.0)),
            );

            if app.needs_focus {
                response.request_focus();
                app.needs_focus = false;
            }

            ui.add_space(2.0);

            for (display_idx, &config_idx) in
                app.filtered_indices.iter().enumerate().take(9)
            {
                let (name, caption): (&str, &str) = match &app.config.entries[config_idx] {
                    crate::config::Entry::Program(p) => (p.name.as_str(), p.caption.as_str()),
                    crate::config::Entry::Group(g) => (g.name.as_str(), g.caption.as_str()),
                };
                let is_selected = display_idx == app.selected_index;
                let text = if caption.is_empty() {
                    format!("{}  {}", display_idx + 1, name)
                } else {
                    format!("{}  {} - {}", display_idx + 1, name, caption)
                };
                let label = egui::SelectableLabel::new(is_selected, text);
                let resp = ui.add(label);
                if resp.clicked() {
                    launch_index = Some(config_idx);
                }
                if resp.secondary_clicked() {
                    edit_index = Some(config_idx);
                }
            }
        });

    // Dynamic window resize
    let row_height = 18.0;
    let height = if visible_count > 0 {
        24.0 + visible_count as f32 * row_height
    } else {
        22.0
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));

    // Handle keyboard
    if escape_pressed {
        app.set_mode(AppMode::Idle);
        return;
    }

    if down_pressed && app.selected_index + 1 < visible_count {
        app.selected_index += 1;
    }
    if up_pressed && app.selected_index > 0 {
        app.selected_index -= 1;
    }

    // Number key launches directly
    if let Some(n) = number_pressed {
        if n < app.filtered_indices.len() {
            launch_index = Some(app.filtered_indices[n]);
        }
    }

    // Ctrl+Enter opens edit for the selected result
    if ctrl_enter {
        if let Some(&config_idx) = app.filtered_indices.get(app.selected_index) {
            edit_index = Some(config_idx);
        }
    } else if enter_pressed {
        if let Some(&config_idx) = app.filtered_indices.get(app.selected_index) {
            launch_index = Some(config_idx);
        } else if !app.search_text.is_empty() {
            app.dialog_name = app.search_text.clone();
            app.dialog_caption.clear();
            app.dialog_executable.clear();
            app.dialog_parameters.clear();
            app.dialog_working_directory.clear();
            app.dialog_error = None;
            app.dialog_return_to_idle = true;
            app.set_mode(AppMode::NewConfig);
            return;
        }
    }

    // Process edit before launch (edit takes priority if both somehow set)
    if let Some(idx) = edit_index {
        open_edit(app, idx);
    } else if let Some(idx) = launch_index {
        app.do_launch(idx);
    }
}
