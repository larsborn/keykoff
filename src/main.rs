#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod focus;
mod hotkey;
mod launcher;
mod tray;
mod ui;

use eframe::egui;
use std::sync::mpsc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1.0, 1.0])
            .with_position([- 10000.0, -10000.0])
            .with_decorations(false)
            .with_taskbar(false),
        ..Default::default()
    };

    // Channels to forward events from handlers (set_event_handler steals from receiver())
    let (menu_tx, menu_rx) = mpsc::channel::<tray_icon::menu::MenuEvent>();
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<global_hotkey::GlobalHotKeyEvent>();

    eframe::run_native(
        "keykoff",
        options,
        Box::new(move |cc| {
            let (tray_icon, tray_state) = tray::create_tray_icon();
            let loaded_config = config::load_config();
            let (hotkey_manager, hotkey_binding) = hotkey::create_hotkey_manager(&loaded_config);

            let ctx1 = cc.egui_ctx.clone();
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                let _ = menu_tx.send(event);
                ctx1.request_repaint();
            }));

            let ctx2 = cc.egui_ctx.clone();
            global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(move |event: global_hotkey::GlobalHotKeyEvent| {
                let _ = hotkey_tx.send(event);
                ctx2.request_repaint();
            }));

            Ok(Box::new(app::KeykoffApp::new(
                loaded_config,
                tray_icon,
                tray_state,
                hotkey_manager,
                hotkey_binding,
                menu_rx,
                hotkey_rx,
            )))
        }),
    )
}
