// src/types.rs

#[derive(Clone, Copy, Debug)]
pub enum MidiMessage {
    NoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        pitch: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
}

pub struct NoteInfo {
    pub channel: u8,
    pub pitch: u8,
    pub velocity: u8,
    pub start_time: f64,
    pub end_time: Option<f64>,
}
