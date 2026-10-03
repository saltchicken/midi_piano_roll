use macroquad::prelude::*;
use std::sync::mpsc;
use rfd::FileDialog;

use crate::constants::*;
use crate::types::{MidiMessage, NoteInfo};
use crate::midi_file::load_midi_file;

pub struct PianoRollApp {
    pub live_notes: Vec<NoteInfo>,
    pub song_notes: Vec<NoteInfo>,
    
    pub playback_time: f64,
    pub is_playing: bool,
    pub playback_speed: f64,
    pub practice_mode: bool,
    pub note_speed_px_per_sec: f32, // Dynamic zoom variable

    // Stores (velocity, count) to fix the polyphony bug
    pub active_pitches: [[(u8, u8); NUM_PITCHES]; NUM_CHANNELS],
    pub cc_values: [[Option<(u8, f64)>; NUM_CONTROLLERS]; NUM_CHANNELS],
    
    pub show_drums: bool,
    pub show_cc: bool,
    pub show_legend: bool,
    pub show_velocity: bool,
}

impl PianoRollApp {
    pub fn new() -> Self {
        Self {
            live_notes: Vec::new(),
            song_notes: Vec::new(),
            playback_time: 0.0,
            is_playing: false,
            playback_speed: 1.0,
            practice_mode: false,
            note_speed_px_per_sec: DEFAULT_NOTE_SPEED,
            active_pitches: [[(0u8, 0u8); NUM_PITCHES]; NUM_CHANNELS],
            cc_values: [[None; NUM_CONTROLLERS]; NUM_CHANNELS],
            show_drums: false,
            show_cc: true,
            show_legend: false,
            show_velocity: false,
        }
    }

    pub fn advance_time(&mut self, dt: f64) {
        if !self.is_playing {
            return;
        }

        if self.practice_mode && self.playback_speed > 0.0 {
            let hit_window = 0.1; // Allowed time variance (seconds) to hit a note early

            // 1. Mark notes as hit if they are in the window and the corresponding key is held
            for note in self.song_notes.iter_mut() {
                if !note.is_hit {
                    if note.channel == DRUM_CHANNEL {
                        // Auto-pass drum notes since they aren't melody keys
                        if note.start_time <= self.playback_time {
                            note.is_hit = true;
                        }
                    } else {
                        // Mark hit if the user is pressing the required pitch
                        if note.start_time <= self.playback_time + hit_window {
                            if Self::is_pitch_active(&self.active_pitches, note.pitch) {
                                note.is_hit = true;
                            }
                        }
                    }
                }
            }

            // 2. Find the earliest un-hit note's start time
            let blocking_time = self.song_notes.iter()
                .filter(|n| !n.is_hit && n.channel != DRUM_CHANNEL)
                .map(|n| n.start_time)
                .fold(f64::INFINITY, |a, b| a.min(b));

            // 3. Advance time but pause (block) if we hit an unplayed note
            if self.playback_time < blocking_time {
                self.playback_time += dt * self.playback_speed;
                if self.playback_time > blocking_time {
                    self.playback_time = blocking_time; // Freeze precisely at the note
                }
            }
        } else {
            // Standard Playback
            self.playback_time += dt * self.playback_speed;
        }
    }

    // Helper to check if a specific pitch is currently being played across any active melodic channel.
    fn is_pitch_active(active_pitches: &[[(u8, u8); NUM_PITCHES]; NUM_CHANNELS], pitch: u8) -> bool {
        for ch in 0..NUM_CHANNELS {
            if ch as u8 == DRUM_CHANNEL { continue; }
            if active_pitches[ch][pitch as usize].1 > 0 {
                return true;
            }
        }
        false
    }
    
    pub fn sync_hits(&mut self) {
        for note in self.song_notes.iter_mut() {
            note.is_hit = note.start_time < self.playback_time;
        }
    }

    pub fn load_song(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("MIDI", &["mid", "midi"]).pick_file() {
            match load_midi_file(path.to_str().unwrap_or("")) {
                Ok(notes) => {
                    self.song_notes = notes;
                    
                    let first_note_t = self.song_notes.first().map(|n| n.start_time).unwrap_or(0.0);
                    self.playback_time = first_note_t - LEAD_IN_SEC;
                    self.is_playing = true;
                }
                Err(e) => eprintln!("Failed to load MIDI: {}", e),
            }
        }
    }

    pub fn update(&mut self, rx: &mpsc::Receiver<MidiMessage>) {
        // Playback Controls
        if is_key_pressed(KeyCode::Space) { self.is_playing = !self.is_playing; }
        if is_key_pressed(KeyCode::O) { self.load_song(); }
        
        // Seek controls
        if is_key_pressed(KeyCode::Left) { self.playback_time -= 5.0; self.sync_hits(); }
        if is_key_pressed(KeyCode::Right) { self.playback_time += 5.0; self.sync_hits(); }

        // Speed controls
        if is_key_pressed(KeyCode::Up) { self.playback_speed += 0.25; }
        if is_key_pressed(KeyCode::Down) { self.playback_speed -= 0.25; }
        if is_key_pressed(KeyCode::R) { self.playback_speed *= -1.0; }
        if is_key_pressed(KeyCode::N) { self.playback_speed = 1.0; }
        
        // Dynamic Zoom (Scroll wheel or +/- keys)
        let (_, mouse_wheel_y) = mouse_wheel();
        if mouse_wheel_y != 0.0 {
            self.note_speed_px_per_sec *= if mouse_wheel_y > 0.0 { 1.1 } else { 0.9 };
        }
        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
            self.note_speed_px_per_sec *= 1.2;
        }
        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
            self.note_speed_px_per_sec /= 1.2;
        }
        self.note_speed_px_per_sec = self.note_speed_px_per_sec.clamp(50.0, 2000.0);

        // View Controls
        if is_key_pressed(KeyCode::P) { self.practice_mode = !self.practice_mode; }
        if is_key_pressed(KeyCode::D) { self.show_drums = !self.show_drums; }
        if is_key_pressed(KeyCode::C) { self.show_cc = !self.show_cc; }
        if is_key_pressed(KeyCode::L) { self.show_legend = !self.show_legend; }
        if is_key_pressed(KeyCode::V) { self.show_velocity = !self.show_velocity; }

        if is_key_pressed(KeyCode::Backspace) {
            self.active_pitches = [[(0u8, 0u8); NUM_PITCHES]; NUM_CHANNELS];
            for note in self.live_notes.iter_mut() {
                if note.end_time.is_none() {
                    note.end_time = Some(self.playback_time);
                }
            }
            self.song_notes.clear();
            self.playback_time = 0.0;
            self.is_playing = false;
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                MidiMessage::NoteOn { channel, pitch, velocity, .. } => {
                    self.live_notes.push(NoteInfo {
                        channel,
                        pitch,
                        velocity,
                        start_time: self.playback_time,
                        end_time: None,
                        is_hit: false,
                    });
                    
                    let state = &mut self.active_pitches[channel as usize][pitch as usize];
                    state.0 = velocity; // update latest velocity
                    state.1 = state.1.saturating_add(1); // increment polyphony count
                }
                MidiMessage::NoteOff { channel, pitch, .. } => {
                    let state = &mut self.active_pitches[channel as usize][pitch as usize];
                    state.1 = state.1.saturating_sub(1);
                    if state.1 == 0 {
                        state.0 = 0; // Clear velocity when no keys of this pitch remain
                    }

                    // Remove .rev() to find and close the OLDEST open note first,
                    // which prevents ghost notes if you mash the same key multiple times quickly.
                    if let Some(note) = self.live_notes
                        .iter_mut()
                        .find(|n| n.pitch == pitch && n.channel == channel && n.end_time.is_none())
                    {
                        note.end_time = Some(self.playback_time);
                    }
                }
                MidiMessage::ControlChange { channel, controller, value, .. } => {
                    self.cc_values[channel as usize][controller as usize] = Some((value, self.playback_time));
                }
            }
        }

        for ch in 0..NUM_CHANNELS {
            for v in self.cc_values[ch].iter_mut() {
                if let Some((_, ts)) = v {
                    if self.playback_time - *ts > CC_HUD_TIMEOUT_SEC {
                        *v = None;
                    }
                }
            }
        }

        let screen_h = screen_height();
        self.live_notes.retain(|n| {
            if let Some(et) = n.end_time {
                ((self.playback_time - et) * self.note_speed_px_per_sec as f64) < screen_h as f64
            } else {
                true
            }
        });
    }
}
