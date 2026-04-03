use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayState {
    pub open_id: tray_icon::menu::MenuId,
    pub edit_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
}

pub fn create_tray_icon() -> (TrayIcon, TrayState) {
    let mut icon_data = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32u8 {
        for x in 0..32u8 {
            // Simple gradient icon: teal with slight variation
            let r = (x * 2).min(60);
            let g = 140 + (y * 3).min(115);
            let b = 180 + (x * 2).min(75);
            icon_data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let icon = Icon::from_rgba(icon_data, 32, 32).unwrap();

    let open_item = MenuItem::new("Open Input", true, None);
    let edit_item = MenuItem::new("Edit Configurations", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let open_id = open_item.id().clone();
    let edit_id = edit_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = Menu::new();
    menu.append_items(&[&open_item, &edit_item, &quit_item])
        .unwrap();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("keykoff")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .build()
        .unwrap();

    (tray, TrayState { open_id, edit_id, quit_id })
}
