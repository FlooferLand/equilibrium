use tray_icon::{MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent, menu::CheckMenuItem};

pub enum TrayUpdate {
    MidiThreadToggle,
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
            .with_icon(Self::load_icon())
            .build()
            .unwrap();

        Self { icon, toggle }
    }
    
    pub fn update(&self) -> TrayUpdate {
        // let window = frame.winit_window().unwrap();
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match &event {
                TrayIconEvent::Click { button_state: MouseButtonState::Down, .. } => {
                    return TrayUpdate::MidiThreadToggle
                },
                _ => {},
            }
        }
        TrayUpdate::None
    }
    
    fn load_icon() -> tray_icon::Icon {
        let path = "./data/icon.png";
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
