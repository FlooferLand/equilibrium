use std::{collections::HashMap, sync::{mpsc::{Receiver, Sender, channel}}, time::{Duration, SystemTime}};
use eframe::{CreationContext, egui::{self, *}};
use crate::{LogKind, app::{text_line::TextLine, tray::Tray}, midi::{self, MidiThreadMessage, midi_thread_main}, platform};

mod tray;
mod text_line;

pub enum AppThreadMessage {
    CloseThread
}

pub struct App {
    midi_receiver: Option<Receiver<MidiThreadMessage>>,
    midi_sender: Option<Sender<AppThreadMessage>>,
    midi_last_contact: SystemTime,
    osd_lines: HashMap<String, TextLine>,
    tray: Tray
}

impl App {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx.set_pixels_per_point(2.0);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::TRANSPARENT;
        visuals.faint_bg_color = Color32::TRANSPARENT;
        cc.egui_ctx.set_visuals(visuals);

        let (app_send, thread_recv) = Self::start_midi_thread();
        Self {
            midi_receiver: Some(thread_recv),
            midi_sender: Some(app_send),
            midi_last_contact: SystemTime::now(),
            osd_lines: HashMap::new(),
            tray: Tray::new()
        }
    }

    pub fn start_midi_thread() -> (Sender<AppThreadMessage>, Receiver<MidiThreadMessage>) {
        let (thread_send, thread_recv) = channel();
        let (app_send, app_recv) = channel();
        std::thread::spawn(move || midi_thread_main(thread_send, app_recv));
        (app_send, thread_recv)
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // OSD lines
        if let Some(midi_receiver) = &self.midi_receiver {
            while let Ok(message) = &midi_receiver.try_recv() {
                match message {
                    MidiThreadMessage::Log(line) => {
                        self.osd_lines.insert(line.text.clone(), TextLine::new(&line.text, line.kind.clone()));
                    }
                    MidiThreadMessage::IncludeChanged { name, enabled } => {
                        let end = if *enabled { "on" } else { "off" };
                        let text = format!("Rack '{name}' is {end}");
                        self.osd_lines.insert(name.clone(), TextLine::new(&text, LogKind::Info));
                    }
                    MidiThreadMessage::Ping => {
                        self.midi_last_contact = SystemTime::now()
                    }
                }
            }
        }
        self.osd_lines.retain(|_, err| !err.is_expired());

        // Tray icon
        match self.tray.update() {
            tray::TrayUpdate::MidiThreadToggle => {
                let running = self.midi_last_contact.elapsed()
                    .unwrap_or(Duration::from_millis(midi::SLEEP_TIME_MILLIS)).as_millis()
                    < midi::SLEEP_TIME_MILLIS as u128 * 2;
                let should_run = !running;
                if should_run {
                    self.osd_lines.insert("midi_thread".to_string(), TextLine::new("Starting the thread", LogKind::Info));
                    let (app_send, thread_recv) = Self::start_midi_thread();
                    self.midi_receiver = Some(thread_recv);
                    self.midi_sender = Some(app_send);
                } else {
                    if let Some(s) = &self.midi_sender {
                        let _ = s.send(AppThreadMessage::CloseThread);
                        self.osd_lines.insert("midi_thread".to_string(), TextLine::new("Closing the thread", LogKind::Info));
                    }
                    self.midi_receiver = None;
                    self.midi_sender = None;
                }
            },
            tray::TrayUpdate::None => {},
        }
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        platform::fix_mouse_passthrough(frame); // TODO: Make this only be called once

        // UI
        if self.osd_lines.is_empty() {
            ui.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return
        }
        ui.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        CentralPanel::default().show(ui, |ui| {
            ui.vertical(|ui| {
                Frame::group(ui.style()).fill(Color32::TRANSPARENT).stroke(Stroke::NONE).show(ui, |ui| {
                    for line in self.osd_lines.values() {
                        let fade = (line.get_fade() * 255f32) as i32;
                        let fade = fade.at_least(0).at_most(255) as u8;
                        let color = match &line.kind {
                            LogKind::Info => Color32::WHITE,
                            LogKind::Warning => Color32::LIGHT_YELLOW,
                            LogKind::Error => Color32::DARK_RED
                        };
                        ui.label(
                            RichText::new(&line.text)
                                .color(Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), fade))
                                .background_color(Color32::BLACK)
                        );
                    }
                });
            });
        });
    }
}