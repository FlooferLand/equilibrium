use std::{path::Path, sync::mpsc::{Receiver, Sender}, time::Duration};

use anyhow::{Context, anyhow, bail};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, backend::cpal::{CpalBackendSettings, cpal::{self, traits::*}}, sound::static_sound::StaticSoundData};
use midir::MidiInput;
use wmidi::MidiMessage;

use crate::{LogKind, app::AppThreadMessage, types::{IncludeData, Keymap, RackFile}};

pub struct LogMessage { pub text: String, pub kind: LogKind }
impl LogMessage {
    pub fn send(sender: &Sender<MidiThreadMessage>, text: String, kind: LogKind) {
        let _ = sender.send(MidiThreadMessage::Log(LogMessage { text, kind }));
    }
}
pub enum MidiThreadMessage {
    Log(LogMessage),
    IncludeChanged { name: String, enabled: bool },
    SoundPlayed { name: String },
    Ping
}

pub const SLEEP_TIME_MILLIS: u64 = 100;
const DEBUG: bool = false;

pub struct Instance {
    sender: Sender<MidiThreadMessage>,
    keymap: Option<Keymap>,
    audio_manager: AudioManager
}
impl Instance {
    fn new(sender: Sender<MidiThreadMessage>) -> anyhow::Result<Self> {
        // Audio
        let host = cpal::default_host();
        let device = match Self::find_audio_device(&host) {
            Ok(device) => {
                println!("Found selected device '{device}'");
                device
            },
            Err(err) => {
                let device = host.default_output_device().unwrap();
                println!("Picking default device '{device}'. Info: {err}");
                device
            },
        };
        let settings = AudioManagerSettings {
            backend_settings: CpalBackendSettings {
                device: Some(device),
                .. Default::default()
            },
            .. Default::default()
        };
        let audio_manager = AudioManager::<DefaultBackend>::new(settings)?;
        
        // Setup
        Ok(Self {
            sender,
            keymap: None,
            audio_manager
        })
    }

    fn find_audio_device(host: &cpal::Host) -> anyhow::Result<cpal::Device> {
        let device_name = std::fs::read_to_string("./data/device.txt").ok()
            .map(|text| text.trim().to_string());

        match device_name {
            Some(name) => {
                let device = host.output_devices()?
                    .find(|d| {
                        d.description()
                            .ok()
                            .map(|desc| desc.name().contains(&name))
                            .unwrap_or(false)
                    }).context("No soundboard audio device found")?;
                Ok(device)
            },
            None => {
                bail!("No device file found. You can make a device.txt inside the data folder to specify the soundboard audio device");
            },
        }
    }

    fn on_message(&mut self, _stamp: u64, message: MidiMessage) -> anyhow::Result<()> {
        let Some(keymap) = &self.keymap else { return Err(anyhow!("Missing keymap")) };
        let MidiMessage::NoteOn(channel, note, _) = message else { return Ok(()) };
        let Some(entry) = keymap.entries.iter().filter_map(|e|
            if let MidiMessage::NoteOn(c, n, _) = e.message && channel == c && note == n {
                Some(e)
            } else {
                None
            }
        ).nth(0) else { return Ok(()) };
        
        match &entry.data {
            IncludeData::Rack(name) => {
                let path = Path::new(&RackFile::get_custom_dir()).to_path_buf().join(&name).with_extension("txt");
                let mut rack = RackFile::load()?;
                let enabled = rack.toggle_include(path.clone())?;
                rack.save()?;

                let _ = self.sender.send(MidiThreadMessage::IncludeChanged { name: name.to_owned(), enabled });
            }
            IncludeData::Sound(name) => {
                let path = Path::new("./data/sounds/").to_path_buf().join(&name);
                let sound = StaticSoundData::from_file(path)?;
                let _ = self.audio_manager.play(sound);
                let _ = self.sender.send(MidiThreadMessage::SoundPlayed { name: name.to_owned() });
            },
        }
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

    let mut instance = Instance::new(sender.clone()).unwrap();
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
                AppThreadMessage::AssetReload => {
                    LogMessage::send(&sender, "MIDI thread should be restarted for some changes".to_string(), LogKind::Info);
                },
            };
        }
        
        let _ = sender.send(MidiThreadMessage::Ping);
        std::thread::sleep(Duration::from_millis(SLEEP_TIME_MILLIS));
    }
}
