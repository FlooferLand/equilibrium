use std::path::{Path, PathBuf};

use anyhow::Context;
use wmidi::{Channel, MidiMessage, Note, U7};

use crate::types::RackFile;

pub struct IncludeEntry {
    pub message: MidiMessage<'static>,
    pub path: PathBuf
}

pub struct Keymap {
    pub entries: Vec<IncludeEntry>
}
impl Keymap {
    pub fn load() -> anyhow::Result<Keymap> {
        let text = std::fs::read_to_string("./data/keymap.txt")?;

        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue }
            
            let (message, name) = Self::parse_line(line)?;
            let path = Path::new(&RackFile::get_custom_dir()).to_path_buf().join(name).with_extension("txt");
            entries.push(IncludeEntry { message, path });
        }
        Ok(Self { entries })
    }

    fn parse_line<'a>(line: &str) -> anyhow::Result<(MidiMessage<'a>, String)> {
        let (message, name) = line.split_once(' ').context("Expected ' '")?;
        let (channel, note) = message.split_once(':').context("Expected ':'")?;
        let name = name.to_owned();

        let channel = channel.parse::<u8>().context("Channel should be a number")?;
        let note = note.parse::<u8>().context("Note should be a number")?;
        
        let channel = Channel::from_index(channel).context("Failed to parse MIDI channel")?;
        let note = Note::from_u8_lossy(note);
        let message = MidiMessage::NoteOn(channel, note, U7::default());
        
        Ok((message, name))
    }
}
