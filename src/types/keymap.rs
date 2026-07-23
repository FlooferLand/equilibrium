use anyhow::{Context, bail};
use wmidi::{Channel, MidiMessage, Note, U7};

pub enum IncludeData {
    Rack(String),
    Sound(String)
}
 
pub struct IncludeEntry {
    pub message: MidiMessage<'static>,
    pub data: IncludeData
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
            entries.push(Self::parse_line(line)?);
        }
        Ok(Self { entries })
    }

    fn parse_line<'a>(line: &str) -> anyhow::Result<IncludeEntry> {
        let line = line.split(' ').collect::<Vec<&str>>();
        if line.len() < 3 { bail!("A keymap line needs 3 sections") }
        let (message, kind, name) = (line[0], line[1], line[2].to_owned());
        let (channel, note) = message.split_once(':').context("Expected ':'")?;

        let channel = channel.parse::<u8>().context("Channel should be a number")?;
        let note = note.parse::<u8>().context("Note should be a number")?;

        let channel = Channel::from_index(channel).context("Failed to parse MIDI channel")?;
        let note = Note::from_u8_lossy(note);
        let message = MidiMessage::NoteOn(channel, note, U7::default());

        let data = match kind {
            "rack" => IncludeData::Rack(name.clone()),
            "sound" => IncludeData::Sound(name.clone()),
            _ => bail!("Unrecognized keymap type '{kind}'")
        };
        
        Ok(IncludeEntry { message, data })
    }
}
