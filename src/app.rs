use macroquad::prelude::*;
use std::sync::mpsc;

use crate::constants::*;
use crate::helpers::{get_channel_color, get_drum_lane, get_key_pos};
use crate::types::{MidiMessage, NoteInfo};

pub struct PianoRollApp {
    notes: Vec<NoteInfo>,
    active_pitches: [[u8; NUM_PITCHES]; NUM_CHANNELS],
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
            notes: Vec::new(),
            active_pitches: [[0u8; NUM_PITCHES]; NUM_CHANNELS],
            cc_values: [[None; NUM_CONTROLLERS]; NUM_CHANNELS],
            show_drums: true,
            show_cc: true,
            show_hints: false,
            show_legend: false,
            show_velocity: false,
        }
    }

    pub fn update(&mut self, rx: &mpsc::Receiver<MidiMessage>, current_time: f64) {
        if is_key_pressed(KeyCode::D) { self.show_drums = !self.show_drums; }
        if is_key_pressed(KeyCode::C) { self.show_cc = !self.show_cc; }
        if is_key_pressed(KeyCode::L) { self.show_legend = !self.show_legend; }
        if is_key_pressed(KeyCode::V) { self.show_velocity = !self.show_velocity; }
        if is_key_pressed(KeyCode::Slash) { self.show_hints = !self.show_hints; }

        if is_key_pressed(KeyCode::Backspace) {
            self.active_pitches = [[0u8; NUM_PITCHES]; NUM_CHANNELS];
            for note in self.notes.iter_mut() {
                if note.end_time.is_none() {
                    note.end_time = Some(current_time);
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                MidiMessage::NoteOn { channel, pitch, velocity, timestamp } => {
                    self.notes.push(NoteInfo {
                        channel,
                        pitch,
                        velocity,
                        start_time: timestamp,
                        end_time: None,
                    });
                    self.active_pitches[channel as usize][pitch as usize] = velocity;
                }
                MidiMessage::NoteOff { channel, pitch, timestamp } => {
                    self.active_pitches[channel as usize][pitch as usize] = 0;
                    if let Some(note) = self.notes
                        .iter_mut()
                        .rev()
                        .find(|n| n.pitch == pitch && n.channel == channel && n.end_time.is_none())
                    {
                        note.end_time = Some(timestamp);
                    }
                }
                MidiMessage::ControlChange { channel, controller, value, timestamp } => {
                    self.cc_values[channel as usize][controller as usize] = Some((value, timestamp));
                }
            }
        }

        for ch in 0..NUM_CHANNELS {
            for v in self.cc_values[ch].iter_mut() {
                if let Some((_, ts)) = v {
                    if current_time - *ts > CC_HUD_TIMEOUT_SEC {
                        *v = None;
                    }
                }
            }
        }

        let screen_h = screen_height();
        self.notes.retain(|n| {
            if let Some(et) = n.end_time {
                ((current_time - et) * NOTE_SPEED_PX_PER_SEC as f64) < screen_h as f64
            } else {
                true
            }
        });
    }

    pub fn draw(&self, current_time: f64) {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        let screen_w = screen_width();
        let screen_h = screen_height();

        let drum_highway_w = if self.show_drums { 300.0_f32.min(screen_w * 0.3) } else { 0.0 };
        let piano_w = screen_w - drum_highway_w;

        let drum_x_start = 0.0;
        let piano_x_start = drum_highway_w;

        let white_key_width = piano_w / NUM_WHITE_KEYS;
        let black_key_width = white_key_width * 0.6;

        self.draw_falling_notes(current_time, screen_h, drum_highway_w, drum_x_start, piano_x_start, white_key_width, black_key_width);
        self.draw_piano_keys(screen_h, piano_x_start, white_key_width, black_key_width);
        
        if self.show_drums {
            self.draw_drum_pads(screen_h, drum_highway_w, drum_x_start);
        }
        
        self.draw_hud(screen_w);
    }

    fn draw_falling_notes(&self, current_time: f64, screen_h: f32, drum_highway_w: f32, drum_x_start: f32, piano_x_start: f32, white_key_width: f32, black_key_width: f32) {
        for note in &self.notes {
            if note.channel == DRUM_CHANNEL {
                if self.show_drums {
                    if let Some((_, lane)) = get_drum_lane(note.pitch) {
                        let lane_w = drum_highway_w / 8.0;
                        let center_x = drum_x_start + (lane as f32 * lane_w) + (lane_w / 2.0);
                        let y = screen_h - KEY_HEIGHT - ((current_time - note.start_time) * NOTE_SPEED_PX_PER_SEC as f64) as f32;

                        if y > screen_h || y < -50.0 { continue; }

                        let color = get_channel_color(note.channel, note.velocity, 1.0);
                        
                        // New blocky representation with solid borders
                        let note_w = lane_w * 0.85;
                        let note_h = 24.0;
                        let x = center_x - note_w / 2.0;
                        let rect_y = y - note_h / 2.0;

                        // Solid base fill
                        draw_rectangle(x, rect_y, note_w, note_h, color);
                        
                        // Solid highlight (no alpha blending) to make it noticeable and pill-like
                        let highlight = Color::new(
                            (color.r + 0.4).min(1.0),
                            (color.g + 0.4).min(1.0),
                            (color.b + 0.4).min(1.0),
                            1.0
                        );
                        draw_rectangle(x + 2.0, rect_y + 2.0, note_w - 4.0, 6.0, highlight);
                        
                        // Thick solid border ensures new notes visibly occlude older ones
                        draw_rectangle_lines(x, rect_y, note_w, note_h, 2.0, BLACK);

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

                let end_t = note.end_time.unwrap_or(current_time);
                let y_bottom = screen_h - KEY_HEIGHT - ((current_time - end_t) * NOTE_SPEED_PX_PER_SEC as f64) as f32;
                let y_top = screen_h - KEY_HEIGHT - ((current_time - note.start_time) * NOTE_SPEED_PX_PER_SEC as f64) as f32;

                let y = y_top;
                let height = (y_bottom - y_top).max(3.0);

                if y > screen_h || y + height < 0.0 { continue; }

                let color = get_channel_color(note.channel, note.velocity, 1.0);
                
                draw_rectangle(x, y, note_width, height, color);
                
                let border_color = Color::new(color.r * 0.6, color.g * 0.6, color.b * 0.6, color.a);
                draw_rectangle_lines(x, y, note_width, height, 1.0, border_color);

                let cap_color = Color::new((color.r * 1.5).min(1.0), (color.g * 1.5).min(1.0), (color.b * 1.5).min(1.0), color.a);
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
                .filter_map(|(p, &vel)| {
                    if vel > 0 && get_drum_lane(p as u8).map(|(_, l)| l) == Some(lane) { Some(vel) } else { None }
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
        let mut cc_text_y = if self.show_hints { 150.0 } else { 30.0 };
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
                "[Backspace] Reset Notes".to_string(),
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
        
        // Draw solid shadow/border to avoid any alpha blending
        draw_text(&vel_text, text_x + 1.0, anchor_y + 1.0, 14.0, BLACK);
        draw_text(&vel_text, text_x, anchor_y, 14.0, WHITE);
    }

    fn get_active_key_color(&self, pitch: u8) -> Option<Color> {
        for ch in 0..NUM_CHANNELS {
            if ch as u8 == DRUM_CHANNEL { continue; }
            let vel = self.active_pitches[ch][pitch as usize];
            if vel > 0 {
                return Some(get_channel_color(ch as u8, vel, 1.0));
            }
        }
        None
    }
}
