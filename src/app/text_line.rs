use std::{fmt::Display, time::{Duration, SystemTime}};

use crate::LogKind;

impl Display for LogKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
        };
        write!(f, "{text}")
    }
}

#[derive(Clone)]
pub struct TextLine {
    pub text: String,
    pub kind: LogKind,
    timestamp: SystemTime
}
impl TextLine {
    pub fn new(text: &str, kind: LogKind) -> Self {
        Self {
            text: text.to_owned(),
            kind: kind.clone(),
            timestamp: SystemTime::now()
        }
    }
    pub fn is_expired(&self) -> bool {
        self.get_elapsed() > Self::max_duration()
    }
    pub fn get_elapsed(&self) -> Duration {
        self.timestamp.elapsed().unwrap_or(Duration::ZERO)
    }
    pub fn get_fade(&self) -> f32 {
        Self::max_duration().checked_sub(self.get_elapsed()).unwrap_or(Self::max_duration()).as_secs_f32() / Self::max_duration().as_secs_f32()
    }

    fn max_duration() -> Duration {
        Duration::from_secs(4)
    }
}
