use std::{collections::HashMap, path::Path, sync::mpsc::Sender, time::Duration};
use anyhow::{Context, bail, anyhow};
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, Tween, backend::cpal::{CpalBackendSettings, cpal::{self, traits::*}}, sound::static_sound::{StaticSoundData, StaticSoundHandle}};
use wmidi::{Channel, MidiMessage, Note};
use crate::{LogKind, midi::{LogMessage, MidiThreadMessage}, types::{IncludeData, Keymap, RackFile}};

struct SoundHandle {
    out: StaticSoundHandle,
    monitor: StaticSoundHandle
}
impl SoundHandle {
    fn act(&mut self, action: impl Fn(&mut StaticSoundHandle)) {
        action(&mut self.out);
        action(&mut self.monitor);
    }
}

pub(super) struct Instance {
    pub sender: Sender<MidiThreadMessage>,
    keymap: Option<Keymap>,
    sounds: HashMap<String, Vec<SoundHandle>>,
    audio_monitor: AudioManager,
    audio_out: AudioManager
}
impl Instance {
    pub fn new(sender: Sender<MidiThreadMessage>) -> anyhow::Result<Self> {
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
        let audio_monitor = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        let audio_out = AudioManager::<DefaultBackend>::new(
            AudioManagerSettings {
                backend_settings: CpalBackendSettings {
                    device: Some(device.clone()),
                    .. Default::default()
                },
                .. Default::default()
            }
        )?;

        // Setup
        let mut instance = Self {
            sender,
            keymap: None,
            audio_monitor,
            audio_out,
            sounds: HashMap::new()
        };
        instance.load_keymap();
        RackFile::load()?.update_visual(&instance.keymap, &mut instance.sender)?;
        Ok(instance)
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

    pub fn on_note(&mut self, channel: Channel, note: Note, velocity: u8) -> anyhow::Result<()> {
        // Checking if the note is mapped
        let Some(keymap) = &self.keymap else { return Err(anyhow!("Missing keymap")) };
        let Some(entry) = keymap.entries.iter().filter_map(|e|
            if let MidiMessage::NoteOn(c, n, _) = e.message && channel == c && note == n {
                Some(e)
            } else {
                None
            }
        ).nth(0) else { return Ok(()) };

        // Action
        match &entry.data {
            IncludeData::Rack(name) => {
                let path = RackFile::build_path(name);
                let mut rack = RackFile::load()?;
                rack.toggle_include(path.clone())?;
                rack.update_visual(&self.keymap, &mut self.sender)?;
                rack.save()?;
            }
            IncludeData::Sound(name) => {
                let path = Path::new("./data/sounds/").to_path_buf().join(&name);
                let tween = Tween { duration: Duration::from_secs_f32(0.1), .. Default::default() };

                let played;
                if velocity > 60 {
                    let sound = StaticSoundData::from_file(path)?;
                    let out = self.audio_out.play(sound.clone())?;
                    let monitor = self.audio_monitor.play(sound)?;
                    let handle = SoundHandle { out, monitor };
                    if let Some(sounds) = self.sounds.get_mut(name) {
                        sounds.push(handle);
                    } else {
                        self.sounds.insert(name.to_owned(), vec![handle]);
                    }
                    played = true;
                } else {
                    if let Some(handles) = self.sounds.get_mut(name) {
                        for handle in handles {
                            handle.act(|h| h.stop(tween));
                        }
                        self.sounds.remove(name);
                    }
                    played = false;
                }

                let _ = self.sender.send(MidiThreadMessage::UpdateSound { name: name.to_owned(), played });
            },
        }
        Ok(())
    }
    
    fn on_pitch_bend(&mut self, _channel: Channel, bend: f64) -> anyhow::Result<()> {
        let tween = Tween { duration: Duration::from_secs_f32(0.1), .. Default::default() };
        for sounds in self.sounds.values_mut() {
            let rate = (bend) + 0.5;
            for sound in sounds {
                sound.act(|h| h.set_playback_rate(rate, tween.clone()));
            }
        }
        Ok(())
    }
    
    fn on_control(&mut self, _channel: Channel, _control: u8, _value: u8) -> anyhow::Result<()> {
        // println!("Control '{control}': {value}");
        Ok(())
    }

    pub fn on_message(&mut self, _stamp: u64, message: MidiMessage) -> anyhow::Result<()> {
        //self.sounds.retain(|_, h| h.out.state() == PlaybackState::Playing);

        match message {
            MidiMessage::NoteOn(channel, note, data) => {
                let velocity = u8::from(data);
                
                // Reset
                if channel.index() == 2 && u8::from(note) == 35 {
                    for sounds in self.sounds.values_mut() {
                        for sound in sounds {
                            sound.act(|h| h.stop(Tween::default()));
                        }
                        LogMessage::send(&self.sender, "Sounds were reset".to_string(), LogKind::Info);
                    }
                    self.sounds.clear();
                }

                self.on_note(channel, note, velocity)?;
            }
            MidiMessage::NoteOff(_channel, _note, _data) => {
            }
            MidiMessage::PitchBendChange(channel, bend) => {
                let bend = (u16::from(bend) as f64 / 8192.0) - 1.0;
                self.on_pitch_bend(channel, bend)?;
            }
            MidiMessage::ControlChange(channel, control, data) => {
                let control = u8::from(control.0);
                let value = u8::from(data);
                self.on_control(channel, control, value)?;
            }
            e => {
                println!("IDK: {e:?}");
            }
        }
        Ok(())
    }
    
    pub fn on_message_debug(&self, _stamp: u64, message: MidiMessage) {
        if let MidiMessage::NoteOn(channel, note, _) = &message {
            println!("{}:{}", channel.index(), u8::from(note.to_owned()));
        }
    }
    
    pub fn load_keymap(&mut self) {
        let result = Keymap::load();
        if let Err(err) = &result {
            let text = format!("Failed to load keymap: {err}");
            LogMessage::send(&self.sender, text, LogKind::Error);
        }
        self.keymap = result.ok();
    }
}
