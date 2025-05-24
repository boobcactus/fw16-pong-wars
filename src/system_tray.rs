use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};
use std::sync::mpsc::{self, Receiver};

pub enum TrayCommand {
    Exit,
}

pub struct SystemTray {
    _tray_icon: TrayIcon,
    rx: Receiver<TrayCommand>,
}

impl SystemTray {
    pub fn new() -> Result<Self> {
        // Create a channel for communication
        let (tx, rx) = mpsc::channel();

        // Create menu items
        let quit_item = MenuItem::new("Exit", true, None);
        let quit_id = quit_item.id().clone();
        
        // Create the menu
        let menu = Menu::new();
        menu.append(&quit_item)?;
        
        // Create a simple 16x16 icon (white square)
        let icon_data = vec![255u8; 16 * 16 * 4]; // RGBA white pixels
        
        // Create the tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("FW16 Pong Wars - Live Mode")
            .with_icon(tray_icon::Icon::from_rgba(icon_data, 16, 16)?)
            .build()?;

        // Spawn a thread to handle menu events
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            let menu_channel = MenuEvent::receiver();
            loop {
                if let Ok(event) = menu_channel.recv() {
                    if event.id == quit_id {
                        let _ = tx_clone.send(TrayCommand::Exit);
                        break;
                    }
                }
            }
        });

        Ok(SystemTray {
            _tray_icon: tray_icon,
            rx,
        })
    }

    pub fn check_commands(&self) -> Option<TrayCommand> {
        self.rx.try_recv().ok()
    }
}
