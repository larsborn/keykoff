use std::sync::mpsc;

use eframe::egui;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIcon;

use crate::config::{self, AppConfig, Entry, RunConfig};
use crate::tray::TrayState;
use crate::ui;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Idle,
    Input,
    NewConfig,
    EditConfig { index: usize },
    NewGroup,
    EditGroup { index: usize },
    ConfigList,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigTab {
    Commands,
    Positioning,
    Hotkey,
    Autostart,
}

impl Default for ConfigTab {
    fn default() -> Self {
        Self::Commands
    }
}

pub struct KeykoffApp {
    pub mode: AppMode,
    mode_changed: bool,
    pub needs_focus: bool,
    focus_requested: bool,
    quit_requested: bool,

    pub config: AppConfig,

    // Input overlay state
    pub search_text: String,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,

    // Config dialog state
    pub dialog_name: String,
    pub dialog_caption: String,
    pub dialog_executable: String,
    pub dialog_parameters: String,
    pub dialog_working_directory: String,
    pub dialog_error: Option<String>,
    pub dialog_return_to_idle: bool,

    // Group dialog state
    pub dialog_members: Vec<String>,
    pub dialog_member_input: String,
    pub dialog_suggestion_index: usize,

    // Config list tab state
    pub config_tab: ConfigTab,

    // System integration (kept alive by being stored)
    _tray_icon: TrayIcon,
    tray_state: TrayState,
    hotkey_manager: GlobalHotKeyManager,
    hotkey: HotKey,

    // Event receivers (forwarded from set_event_handler callbacks)
    menu_rx: mpsc::Receiver<MenuEvent>,
    hotkey_rx: mpsc::Receiver<GlobalHotKeyEvent>,
}

impl KeykoffApp {
    pub fn new(
        tray_icon: TrayIcon,
        tray_state: TrayState,
        hotkey_manager: GlobalHotKeyManager,
        hotkey: HotKey,
        menu_rx: mpsc::Receiver<MenuEvent>,
        hotkey_rx: mpsc::Receiver<GlobalHotKeyEvent>,
    ) -> Self {
        Self {
            mode: AppMode::Idle,
            mode_changed: true,
            needs_focus: false,
            focus_requested: false,
            quit_requested: false,
            config: config::load_config(),
            search_text: String::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            dialog_name: String::new(),
            dialog_caption: String::new(),
            dialog_executable: String::new(),
            dialog_parameters: String::new(),
            dialog_working_directory: String::new(),
            dialog_error: None,
            dialog_return_to_idle: false,
            dialog_members: Vec::new(),
            dialog_member_input: String::new(),
            dialog_suggestion_index: 0,
            config_tab: ConfigTab::default(),
            _tray_icon: tray_icon,
            tray_state,
            hotkey_manager: hotkey_manager,
            hotkey,
            menu_rx,
            hotkey_rx,
        }
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        match &mode {
            AppMode::Input => {
                self.search_text.clear();
                self.filtered_indices.clear();
                self.selected_index = 0;
                self.needs_focus = true;
            }
            AppMode::NewConfig | AppMode::EditConfig { .. }
            | AppMode::NewGroup | AppMode::EditGroup { .. } => {
                self.needs_focus = true;
            }
            _ => {}
        }
        self.mode = mode;
        self.mode_changed = true;
    }

    pub fn update_filtered_results(&mut self) {
        let query = self.search_text.to_lowercase();
        if query.is_empty() {
            self.filtered_indices.clear();
        } else {
            self.filtered_indices = self
                .config
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| crate::config::entry_name(e).to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
        }
    }

    pub fn do_launch(&mut self, config_index: usize) {
        use crate::config::{flatten_group_to_programs, Entry};
        match self.config.entries.get(config_index) {
            Some(Entry::Program(_)) => {
                // Clone the program so we can subsequently call &mut self methods
                // (e.g. populate_program_dialog_from_index) without a borrow conflict.
                let program = if let Some(Entry::Program(p)) = self.config.entries.get(config_index) {
                    p.clone()
                } else {
                    return;
                };
                if let Err(e) = crate::launcher::launch(&program) {
                    self.populate_program_dialog_from_index(config_index);
                    self.dialog_error = Some(e);
                    self.dialog_return_to_idle = true;
                    self.set_mode(AppMode::EditConfig { index: config_index });
                } else {
                    self.set_mode(AppMode::Idle);
                }
            }
            Some(Entry::Group(_)) => {
                let program_indices = flatten_group_to_programs(&self.config.entries, config_index);
                for idx in program_indices {
                    if let Some(Entry::Program(p)) = self.config.entries.get(idx) {
                        if let Err(e) = crate::launcher::launch(p) {
                            eprintln!("group launch: '{}' failed: {}", p.name, e);
                        }
                    }
                }
                self.set_mode(AppMode::Idle);
            }
            None => {
                self.set_mode(AppMode::Idle);
            }
        }
    }

    pub fn populate_program_dialog_from_index(&mut self, index: usize) {
        if let Some(crate::config::Entry::Program(p)) = self.config.entries.get(index) {
            self.dialog_name = p.name.clone();
            self.dialog_caption = p.caption.clone();
            self.dialog_executable = p.executable.clone();
            self.dialog_parameters = p.parameters.clone();
            self.dialog_working_directory = p.working_directory.clone();
            self.dialog_error = None;
        }
    }

    pub fn clear_dialog_fields(&mut self) {
        self.dialog_name.clear();
        self.dialog_caption.clear();
        self.dialog_executable.clear();
        self.dialog_parameters.clear();
        self.dialog_working_directory.clear();
        self.dialog_error = None;
    }

    pub fn clear_group_dialog_fields(&mut self) {
        self.dialog_name.clear();
        self.dialog_caption.clear();
        self.dialog_members.clear();
        self.dialog_member_input.clear();
        self.dialog_suggestion_index = 0;
        self.dialog_error = None;
    }

    pub fn populate_group_dialog_from_index(&mut self, index: usize) {
        if let Some(crate::config::Entry::Group(g)) = self.config.entries.get(index) {
            self.dialog_name = g.name.clone();
            self.dialog_caption = g.caption.clone();
            self.dialog_members = g.members.clone();
            self.dialog_member_input.clear();
            self.dialog_suggestion_index = 0;
            self.dialog_error = None;
        }
    }

    pub fn save_dialog_entry(&mut self) -> bool {
        if self.dialog_name.trim().is_empty() {
            self.dialog_error = Some("Name is required.".into());
            return false;
        }
        if self.dialog_executable.trim().is_empty() {
            self.dialog_error = Some("Executable path is required.".into());
            return false;
        }

        let program = RunConfig {
            name: self.dialog_name.trim().to_string(),
            caption: self.dialog_caption.trim().to_string(),
            executable: self.dialog_executable.trim().trim_matches('"').to_string(),
            parameters: self.dialog_parameters.trim().to_string(),
            working_directory: self.dialog_working_directory.trim().to_string(),
        };

        match &self.mode {
            AppMode::NewConfig => self.config.entries.push(Entry::Program(program)),
            AppMode::EditConfig { index } => self.config.entries[*index] = Entry::Program(program),
            _ => {}
        }

        if let Err(e) = config::save_config(&self.config) {
            self.dialog_error = Some(format!("Failed to save: {}", e));
            return false;
        }

        true
    }

    pub fn reregister_hotkey(&mut self) {
        let _ = self.hotkey_manager.unregister(self.hotkey);
        self.hotkey = crate::hotkey::hotkey_from_config(&self.config);
        let _ = self.hotkey_manager.register(self.hotkey);
    }

    fn apply_mode_viewport_commands(&mut self, ctx: &egui::Context) {
        if !self.mode_changed {
            return;
        }
        self.mode_changed = false;

        match &self.mode {
            AppMode::Idle => {
                // Don't use Visible(false) — it freezes the event loop on Windows.
                // Instead, park the window off-screen so events keep processing.
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1.0, 1.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    -10000.0, -10000.0,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::Normal,
                ));
            }
            AppMode::Input => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(self.config.overlay_width, 22.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    self.config.overlay_x, self.config.overlay_y,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::AlwaysOnTop,
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            AppMode::NewConfig | AppMode::EditConfig { .. }
            | AppMode::NewGroup | AppMode::EditGroup { .. } => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(500.0, 280.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(350.0, 250.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::Normal,
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    400.0, 300.0,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            AppMode::ConfigList => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(600.0, 400.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::Normal,
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    350.0, 250.0,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
    }

    fn handle_events(&mut self) {
        // Drain all queued tray menu events
        while let Ok(event) = self.menu_rx.try_recv() {
            if event.id == self.tray_state.open_id {
                self.set_mode(AppMode::Input);
            } else if event.id == self.tray_state.edit_id {
                self.set_mode(AppMode::ConfigList);
            } else if event.id == self.tray_state.quit_id {
                self.quit_requested = true;
            }
        }

        // Drain all queued hotkey events (only act on Pressed, ignore Released)
        while let Ok(event) = self.hotkey_rx.try_recv() {
            if event.id == self.hotkey.id() && event.state == HotKeyState::Pressed {
                match self.mode {
                    AppMode::Idle => self.set_mode(AppMode::Input),
                    AppMode::Input => self.set_mode(AppMode::Idle),
                    _ => self.focus_requested = true,
                }
            }
        }
    }
}

impl eframe::App for KeykoffApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_mode_viewport_commands(ctx);
        self.handle_events();

        if self.focus_requested {
            self.focus_requested = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        // Intercept window close: hide instead of quit, unless quit was requested
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.quit_requested {
                // Let the close proceed — don't cancel
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.set_mode(AppMode::Idle);
            return;
        }

        // Handle quit from tray
        if self.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let mode = self.mode.clone();
        match mode {
            AppMode::Idle => {
                // Keep polling events while hidden
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
            AppMode::Input => {
                ui::input_overlay::show(self, ctx);
            }
            AppMode::NewConfig | AppMode::EditConfig { .. } => {
                ui::config_dialog::show(self, ctx);
            }
            AppMode::NewGroup | AppMode::EditGroup { .. } => {
                ui::group_dialog::show(self, ctx);
            }
            AppMode::ConfigList => {
                ui::config_list::show(self, ctx);
            }
        }
    }
}
