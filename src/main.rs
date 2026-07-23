#![allow(dead_code)]
use std::{cell::RefCell, rc::Rc};

use eframe::egui::{Pos2, Vec2, ViewportBuilder, WindowLevel};
use tray_icon::TrayIconBuilder;

use crate::app::App;

mod platform;
mod types;
mod midi;
mod app;

#[derive(Clone)]
pub enum LogKind {
    Info,
    Warning,
    Error
}

fn main() -> eframe::Result<()> {
    env_logger::builder().init();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_position(Pos2::ZERO)
            .with_inner_size(Vec2::new(800.0, 600.0))
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_has_shadow(false)
            .with_mouse_passthrough(true)
            .with_taskbar(false)
            .with_window_level(WindowLevel::AlwaysOnTop),
        .. Default::default()
    };
    eframe::run_native(
        "Equilibrium",
        options,
        Box::new(|cc| {
            Ok(Box::new(App::new(cc)))
        })
    )
}