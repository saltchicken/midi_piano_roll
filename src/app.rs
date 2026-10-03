use macroquad::prelude::*;
use std::sync::mpsc;
use rfd::FileDialog;

use crate::constants::*;
use crate::helpers::{get_channel_color, get_drum_lane, get_key_pos};
use crate::types::{MidiMessage, NoteInfo};
use crate::midi_file::load_midi_file;

pub struct PianoRollApp {
    live_notes: Vec<NoteInfo>,
    pub song_notes: Vec<NoteInfo>,
    
    pub playback_time: f64,
    pub is_playing: bool,
    pub playback_speed: f64,
    pub practice_mode: bool,
    pub note_speed_px_per_sec: f32, // Dynamic zoom variable

    // Stores (velocity, count) to fix the polyphony bug
    active_pitches: [[(u8, u8); NUM_PITCHES]; NUM_CHANNELS],
    cc_values: [[Option<(u8, f64)>; NUM_CONTROLLERS]; NUM_CHANNELS],
    
    show_drums: bool,
    show_cc: bool,
    show_hints: bool,
    show_legend: bool,
    show_velocity: bool,
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
            show_drums: true,
            show_cc: true,
            show_hints: true,
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
        if is_key_pressed(KeyCode::Slash) { self.show_hints = !self.show_hints; }

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

    pub fn draw(&self) {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        let screen_w = screen_width();
        let screen_h = screen_height();

        let drum_highway_w = if self.show_drums { 300.0_f32.min(screen_w * 0.3) } else { 0.0 };
        let piano_w = screen_w - drum_highway_w;

        let drum_x_start = 0.0;
        let piano_x_start = drum_highway_w;

        let white_key_width = piano_w / NUM_WHITE_KEYS;
        let black_key_width = white_key_width * 0.6;

        self.draw_falling_notes(screen_h, drum_highway_w, drum_x_start, piano_x_start, white_key_width, black_key_width);
        self.draw_piano_keys(screen_h, piano_x_start, white_key_width, black_key_width);
        
        if self.show_drums {
            self.draw_drum_pads(screen_h, drum_highway_w, drum_x_start);
        }
        
        self.draw_hud(screen_w);
    }

    fn draw_falling_notes(&self, screen_h: f32, drum_highway_w: f32, drum_x_start: f32, piano_x_start: f32, white_key_width: f32, black_key_width: f32) {
        let key_y = screen_h - KEY_HEIGHT;

        for note in self.song_notes.iter().chain(self.live_notes.iter()) {
            let alpha = if self.practice_mode && note.is_hit { 0.3 } else { 1.0 };

            if note.channel == DRUM_CHANNEL {
                if self.show_drums {
                    if let Some((_, lane)) = get_drum_lane(note.pitch) {
                        let lane_w = drum_highway_w / 8.0;
                        let center_x = drum_x_start + (lane as f32 * lane_w) + (lane_w / 2.0);
                        
                        let time_until_hit = note.start_time - self.playback_time;
                        let y = key_y - (time_until_hit * self.note_speed_px_per_sec as f64) as f32;

                        if y > screen_h || y < -50.0 { continue; }

                        let color = get_channel_color(note.channel, note.velocity, alpha);
                        let note_w = lane_w * 0.85;
                        let note_h = 24.0;
                        let x = center_x - note_w / 2.0;
                        let rect_y = y - note_h / 2.0;

                        draw_rectangle(x, rect_y, note_w, note_h, color);
                        
                        let highlight = Color::new(
                            (color.r + 0.4).min(1.0),
                            (color.g + 0.4).min(1.0),
                            (color.b + 0.4).min(1.0),
                            alpha
                        );
                        draw_rectangle(x + 2.0, rect_y + 2.0, note_w - 4.0, 6.0, highlight);
                        draw_rectangle_lines(x, rect_y, note_w, note_h, 2.0, Color::new(0.0, 0.0, 0.0, alpha));

                        if self.show_velocity {
                            self.draw_velocity_text(note.velocity, center_x, rect_y - 6.0);
                        }
                    }
                }
            } else {
                let (is_black, white_idx) = get_key_pos(note.pitch);
                let center_x = piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);
                
                let note_width = if is_black { black_key_width } else { white_key_width - 2.0 };
                let x = center_x - (note_width / 2.0);

                let end_t = note.end_time.unwrap_or(self.playback_time);
                
                let time_until_hit = note.start_time - self.playback_time;
                let time_until_end = end_t - self.playback_time;

                let y_bottom = key_y - (time_until_hit * self.note_speed_px_per_sec as f64) as f32;
                let y_top = key_y - (time_until_end * self.note_speed_px_per_sec as f64) as f32;

                let y = y_top;
                let height = (y_bottom - y_top).max(3.0);

                if y > screen_h || y + height < 0.0 { continue; }

                let color = get_channel_color(note.channel, note.velocity, alpha);
                
                draw_rectangle(x, y, note_width, height, color);
                
                let border_color = Color::new(color.r * 0.6, color.g * 0.6, color.b * 0.6, alpha);
                draw_rectangle_lines(x, y, note_width, height, 1.0, border_color);

                let cap_color = Color::new((color.r * 1.5).min(1.0), (color.g * 1.5).min(1.0), (color.b * 1.5).min(1.0), alpha);
                draw_rectangle(x, y + height - 2.0, note_width, 2.0, cap_color);

                if self.show_velocity { 
                    self.draw_velocity_text(note.velocity, center_x, y + height - 3.0);
                }
            }
        }
    }

    fn draw_piano_keys(&self, screen_h: f32, piano_x_start: f32, white_key_width: f32, black_key_width: f32) {
        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if !is_black {
                let color = self.get_active_key_color(i).unwrap_or(BLACK);
                let x = piano_x_start + white_idx * white_key_width;
                
                draw_rectangle(x, screen_h - KEY_HEIGHT, white_key_width, KEY_HEIGHT, color);
                draw_rectangle_lines(x, screen_h - KEY_HEIGHT, white_key_width, KEY_HEIGHT, 1.0, Color::new(0.2, 0.2, 0.2, 1.0));
            }
        }

        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if is_black {
                let color = self.get_active_key_color(i).unwrap_or(Color::new(0.1, 0.1, 0.1, 1.0));
                let center_x = piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);
                let x = center_x - (black_key_width / 2.0);
                
                draw_rectangle(x, screen_h - KEY_HEIGHT + 1.0, black_key_width, KEY_HEIGHT * 0.65, color);
            }
        }
    }

    fn draw_drum_pads(&self, screen_h: f32, drum_highway_w: f32, drum_x_start: f32) {
        let lane_w = drum_highway_w / 8.0;
        for lane in 0..8 {
            let x = drum_x_start + (lane as f32 * lane_w);

            let max_vel = self.active_pitches[DRUM_CHANNEL as usize]
                .iter()
                .enumerate()
                .filter_map(|(p, &(vel, count))| {
                    if count > 0 && get_drum_lane(p as u8).map(|(_, l)| l) == Some(lane) { Some(vel) } else { None }
                })
                .max();

            let color = if let Some(vel) = max_vel {
                get_channel_color(DRUM_CHANNEL, vel, 1.0)
            } else {
                Color::new(0.2, 0.2, 0.2, 1.0)
            };

            draw_rectangle(x, screen_h - KEY_HEIGHT, lane_w - 2.0, KEY_HEIGHT, color);
            draw_rectangle_lines(x, screen_h - KEY_HEIGHT, lane_w - 2.0, KEY_HEIGHT, 1.0, GRAY);

            let label = match lane {
                0 => "KICK", 1 => "SNR", 2 => "CHH", 3 => "OHH", 
                4 => "TOM", 5 => "CRSH", 6 => "RIDE", _ => "PERC",
            };

            let text_size = measure_text(label, None, 16u16, 1.0);
            let text_x = x + (lane_w / 2.0) - (text_size.width / 2.0);
            draw_text(label, text_x, screen_h - (KEY_HEIGHT / 2.0) + (text_size.height / 2.0), 16.0, WHITE);
        }
    }

    fn draw_hud(&self, screen_w: f32) {
        // Draw Playback Time at Top Center
        let time_str = format!("Time: {:.1}s ({:.2}x)", self.playback_time, self.playback_speed);
        let time_size = measure_text(&time_str, None, 24, 1.0);
        let time_color = if self.practice_mode { ORANGE } else if self.is_playing { GREEN } else { YELLOW };
        draw_text(&time_str, (screen_w / 2.0) - (time_size.width / 2.0), 30.0, 24.0, time_color);

        let mut cc_text_y = if self.show_hints { 280.0 } else { 30.0 };
        let cc_text_x = screen_w - 280.0;

        let ccs_active = self.cc_values.iter().any(|ch_array| ch_array.iter().any(|v| v.is_some()));

        if self.show_cc && ccs_active {
            draw_text("MIDI CC Monitor", cc_text_x, cc_text_y, 20.0, WHITE);
            cc_text_y += 25.0;

            for ch in 0..NUM_CHANNELS {
                for (controller, value_opt) in self.cc_values[ch].iter().enumerate() {
                    if let Some((value, _)) = *value_opt {
                        let text = format!("CH {:02} | CC {:03}: {:03}", ch + 1, controller, value);
                        draw_text(&text, cc_text_x, cc_text_y, 16.0, LIGHTGRAY);

                        let bar_width = 100.0;
                        let fill_width = (value as f32 / 127.0) * bar_width;
                        let bar_x = cc_text_x + 145.0;

                        draw_rectangle(bar_x, cc_text_y - 12.0, bar_width, 10.0, Color::new(0.2, 0.2, 0.2, 0.8));
                        draw_rectangle(bar_x, cc_text_y - 12.0, fill_width, 10.0, Color::new(0.0, 0.8, 0.5, 0.8));

                        cc_text_y += 20.0;
                    }
                }
            }
        }

        if self.show_hints {
            let hints = [
                "[?] Toggle Hints".to_string(),
                "[O] Load MIDI File".to_string(),
                "[Space] Play/Pause Song".to_string(),
                "[P] Practice Mode: ".to_string() + if self.practice_mode { "ON" } else { "OFF" },
                "[<] [>] Seek Time".to_string(),
                "[^] [v] Adjust Speed".to_string(),
                "[Scroll / + -] Zoom In/Out".to_string(),
                "[R] Reverse Direction".to_string(),
                "[N] Normal Speed (1.0x)".to_string(),
                "[Backspace] Clear Notes".to_string(),
                format!("[D] Drums: {}", if self.show_drums { "ON" } else { "OFF" }),
                format!("[C] CC Monitor: {}", if self.show_cc { "ON" } else { "OFF" }),
                format!("[L] Legend: {}", if self.show_legend { "ON" } else { "OFF" }),
                format!("[V] Velocity: {}", if self.show_velocity { "ON" } else { "OFF" })
            ];

            let max_w = hints.iter().map(|h| measure_text(h, None, 20, 1.0).width).fold(0.0, f32::max);
            let hints_x = screen_w - max_w - 15.0;
            let mut hints_y = 30.0;

            for hint in hints {
                draw_text(&hint, hints_x, hints_y, 20.0, WHITE);
                hints_y += 25.0;
            }
        }

        if self.show_legend {
            let legend_x = 20.0;
            let mut legend_y = 30.0;

            draw_rectangle(legend_x - 10.0, legend_y - 20.0, 150.0, 390.0, Color::new(0.0, 0.0, 0.0, 0.6));
            draw_text("Channels", legend_x, legend_y, 20.0, WHITE);
            legend_y += 15.0;

            for ch in 0..NUM_CHANNELS as u8 {
                let color = get_channel_color(ch, 127, 1.0);
                draw_rectangle(legend_x, legend_y, 16.0, 16.0, color);
                draw_rectangle_lines(legend_x, legend_y, 16.0, 16.0, 1.0, GRAY);
                
                let label = if ch == DRUM_CHANNEL { format!("CH 10 (Drums)") } else { format!("CH {}", ch + 1) };
                draw_text(&label, legend_x + 25.0, legend_y + 13.0, 16.0, WHITE);
                legend_y += 22.0;
            }
        }
    }

    fn draw_velocity_text(&self, velocity: u8, anchor_x: f32, anchor_y: f32) {
        let vel_text = format!("{}", velocity);
        let text_size = measure_text(&vel_text, None, 14, 1.0);
        let text_x = anchor_x - (text_size.width / 2.0);
        
        draw_text(&vel_text, text_x + 1.0, anchor_y + 1.0, 14.0, BLACK);
        draw_text(&vel_text, text_x, anchor_y, 14.0, WHITE);
    }

    fn get_active_key_color(&self, pitch: u8) -> Option<Color> {
        for ch in 0..NUM_CHANNELS {
            if ch as u8 == DRUM_CHANNEL { continue; }
            let (vel, count) = self.active_pitches[ch][pitch as usize];
            if count > 0 {
                return Some(get_channel_color(ch as u8, vel, 1.0));
            }
        }
        None
    }
}
