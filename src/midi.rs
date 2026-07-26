use std::{path::PathBuf, sync::mpsc::{Receiver, Sender}, time::Duration};
use midir::MidiInput;
use crate::{LogKind, app::AppThreadMessage, midi::instance::*};

mod instance;

pub struct LogMessage { pub text: String, pub kind: LogKind }
impl LogMessage {
    pub fn send(sender: &Sender<MidiThreadMessage>, text: String, kind: LogKind) {
        let _ = sender.send(MidiThreadMessage::Log(LogMessage { text, kind }));
    }
}
pub enum MidiThreadMessage {
    Log(LogMessage),
    UpdateRack { path: PathBuf, enabled: bool, in_keymap: bool },
    UpdateSound { name: String, played: bool },
    Ping
}

pub const SLEEP_TIME_MILLIS: u64 = 100;
const DEBUG: bool = false;

// Main
pub fn midi_thread_main(sender: Sender<MidiThreadMessage>, receiver: Receiver<AppThreadMessage>) {
    let input = MidiInput::new("Equilibrium").unwrap();
    let ports = input.ports();
    let port = ports
        .iter()
        .find(|p| input.port_name(p).unwrap().contains("MPK"))
        .expect("No MIDI controller found");
    
    LogMessage::send(&sender, format!("Found port '{}'", port.id()), LogKind::Info);

    let instance = Instance::new(sender.clone()).unwrap();
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
                AppThreadMessage::AssetReload => {
                    LogMessage::send(&sender, "MIDI thread should be restarted for some changes".to_string(), LogKind::Info);
                },
            };
        }
        
        let _ = sender.send(MidiThreadMessage::Ping);
        std::thread::sleep(Duration::from_millis(SLEEP_TIME_MILLIS));
    }
}
