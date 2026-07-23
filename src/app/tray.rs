use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent, menu::CheckMenuItem};

pub enum TrayUpdate {
    MidiThreadToggle,
    AssetReload,
    None
}

// Should run on it's own separate thread
// Popping up a menu freezes egui for some stupid reason

pub struct Tray {
    icon: TrayIcon,
    toggle: CheckMenuItem
}

impl Tray {
    pub fn new() -> Self {
        let toggle = CheckMenuItem::new("Running", true, true, None);
        let icon = TrayIconBuilder::new()
            .with_icon(Self::load_icon("open"))
            .build()
            .unwrap();

        Self { icon, toggle }
    }
    
    pub fn update(&self, running: bool) -> TrayUpdate {
        let _ = self.icon.set_tooltip(Some(
            if running { "Running" } else { "Not running" }.to_owned()
        ));
        let _ = self.icon.set_icon(Some(
            Self::load_icon(if running { "open" } else { "closed" })
        ));

        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match &event {
                TrayIconEvent::Click { button_state: MouseButtonState::Down, button: MouseButton::Left, .. } => {
                    return TrayUpdate::MidiThreadToggle
                },
                TrayIconEvent::Click { button_state: MouseButtonState::Down, button: MouseButton::Right, .. } => {
                    return TrayUpdate::AssetReload
                },
                _ => {},
            }
        }
        TrayUpdate::None
    }
    
    fn load_icon(icon: &str) -> tray_icon::Icon {
        let path = format!("./assets/icon_{icon}.png");
        let (icon_rgba, icon_width, icon_height) = {
            let image = image::open(path)
                .expect("Failed to open icon path")
                .into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    }
}
