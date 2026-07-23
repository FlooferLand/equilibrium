use std::{sync::{mpsc::{Receiver, Sender}}, time::Duration};

use anyhow::anyhow;
use midir::MidiInput;
use wmidi::MidiMessage;

use crate::{LogKind, app::AppThreadMessage, types::{Keymap, RackFile}};

pub struct LogMessage { pub text: String, pub kind: LogKind }
impl LogMessage {
    pub fn send(sender: &Sender<MidiThreadMessage>, text: String, kind: LogKind) {
        let _ = sender.send(MidiThreadMessage::Log(LogMessage { text, kind }));
    }
}
pub enum MidiThreadMessage {
    Log(LogMessage),
    IncludeChanged { name: String, enabled: bool },
    Ping
}

pub const SLEEP_TIME_MILLIS: u64 = 100;
const DEBUG: bool = false;

pub struct Instance {
    sender: Sender<MidiThreadMessage>,
    keymap: Option<Keymap>
}
impl Instance {
    fn on_message(&self, _stamp: u64, message: MidiMessage) -> anyhow::Result<()> {
        let Some(keymap) = &self.keymap else { return Err(anyhow!("Missing keymap")) };
        let MidiMessage::NoteOn(channel, note, _) = message else { return Ok(()) };
        let Some(entry) = keymap.entries.iter().filter_map(|e|
            if let MidiMessage::NoteOn(c, n, _) = e.message && channel == c && note == n {
                Some(e)
            } else {
                None
            }
        ).nth(0) else { return Ok(()) };
        
        let mut rack = RackFile::load()?;
        let enabled = rack.toggle_include(entry.path.clone())?;
        rack.save()?;

        let name = entry.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or(entry.path.display().to_string());
        let _ = self.sender.send(MidiThreadMessage::IncludeChanged { name, enabled });
        Ok(())
    }
    fn on_message_debug(&self, _stamp: u64, message: MidiMessage) {
        if let MidiMessage::NoteOn(channel, note, _) = &message {
            println!("{}:{}", channel.index(), u8::from(note.to_owned()));
        }
    }
    
    fn load_keymap(&mut self) {
        let result = Keymap::load();
        if let Err(err) = &result {
            let text = format!("Failed to load keymap: {err}");
            LogMessage::send(&self.sender, text, LogKind::Error);
        }
        self.keymap = result.ok();
    }
}

pub fn midi_thread_main(sender: Sender<MidiThreadMessage>, receiver: Receiver<AppThreadMessage>) {
    let input = MidiInput::new("Equilibrium").unwrap();
    let ports = input.ports();
    let port = ports
        .iter()
        .find(|p| input.port_name(p).unwrap().contains("MPK"))
        .expect("No MIDI controller found");
    
    LogMessage::send(&sender, format!("Found port '{}'", port.id()), LogKind::Info);

    let mut instance = Instance {
        sender: sender.clone(),
        keymap: None
    };
    instance.load_keymap();
    let connection = input.connect(port, "equilibrium_read", move |stamp, message, instance| {
        match wmidi::MidiMessage::from_bytes(message) {
            Ok(message) => {
                if DEBUG { instance.on_message_debug(stamp, message.clone()); }
                let Err(err) = instance.on_message(stamp, message) else { return };
                LogMessage::send(&instance.sender, err.to_string(), LogKind::Error);
            },
            Err(err) => {
                let err = format!(
                    "Failed to read MIDI message '{}': {err}",
                    message.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", ")
                );
                LogMessage::send(&instance.sender, err, LogKind::Error);
            },
        }
    }, instance).unwrap();

    loop {
        if let Ok(message) = receiver.try_recv() {
            match message {
                AppThreadMessage::CloseThread => {
                    drop(connection);
                    return;
                }
            };
        }
        
        let _ = sender.send(MidiThreadMessage::Ping);
        std::thread::sleep(Duration::from_millis(SLEEP_TIME_MILLIS));
    }
}
