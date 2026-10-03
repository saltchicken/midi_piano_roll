use macroquad::prelude::*;

use crate::app::PianoRollApp;
use crate::constants::*;
use crate::helpers::{get_channel_color, get_drum_lane, get_key_pos};

pub fn draw(app: &PianoRollApp) {
    clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

    let screen_w = screen_width();
    let screen_h = screen_height();

    let drum_highway_w = if app.show_drums { 300.0_f32.min(screen_w * 0.3) } else { 0.0 };
    let piano_w = screen_w - drum_highway_w;

    let drum_x_start = 0.0;
    let piano_x_start = drum_highway_w;

    let white_key_width = piano_w / NUM_WHITE_KEYS;
    let black_key_width = white_key_width * 0.6;

    draw_falling_notes(app, screen_h, drum_highway_w, drum_x_start, piano_x_start, white_key_width, black_key_width);
    draw_piano_keys(app, screen_h, piano_x_start, white_key_width, black_key_width);
    
    if app.show_drums {
        draw_drum_pads(app, screen_h, drum_highway_w, drum_x_start);
    }
    
    draw_hud(app, screen_w);
}

fn draw_falling_notes(app: &PianoRollApp, screen_h: f32, drum_highway_w: f32, drum_x_start: f32, piano_x_start: f32, white_key_width: f32, black_key_width: f32) {
    let key_y = screen_h - KEY_HEIGHT;

    for note in app.song_notes.iter().chain(app.live_notes.iter()) {
        let alpha = if app.practice_mode && note.is_hit { 0.3 } else { 1.0 };

        if note.channel == DRUM_CHANNEL {
            if app.show_drums {
                if let Some((_, lane)) = get_drum_lane(note.pitch) {
                    let lane_w = drum_highway_w / 8.0;
                    let center_x = drum_x_start + (lane as f32 * lane_w) + (lane_w / 2.0);
                    
                    let time_until_hit = note.start_time - app.playback_time;
                    let y = key_y - (time_until_hit * app.note_speed_px_per_sec as f64) as f32;

                    if y > screen_h || y < -50.0 { continue; }

                    let color = get_channel_color(note.channel, note.velocity, alpha);
                    let note_w = lane_w * 0.85;
                    let note_h = 24.0;
                    let x = center_x - note_w / 2.0;
                    let rect_y = y - note_h / 2.0;

                    draw_rectangle(x, rect_y, note_w, note_h, color);
                    
                    let highlight = Color::new(
                        (color.r + 0.4).min(1.0), (color.g + 0.4).min(1.0),
                        (color.b + 0.4).min(1.0), alpha
                    );
                    
                    draw_rectangle(x + 2.0, rect_y + 2.0, note_w - 4.0, 6.0, highlight);
                    draw_rectangle_lines(x, rect_y, note_w, note_h, 2.0, Color::new(0.0, 0.0, 0.0, alpha));

                    if app.show_velocity {
                        draw_velocity_text(note.velocity, center_x, rect_y - 6.0);
                    }
                }
            }
        } else {
            let (is_black, white_idx) = get_key_pos(note.pitch);
            let center_x = piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);
            
            let note_width = if is_black { black_key_width } else { white_key_width - 2.0 };
            let x = center_x - (note_width / 2.0);

            let end_t = note.end_time.unwrap_or(app.playback_time);
            
            let time_until_hit = note.start_time - app.playback_time;
            let time_until_end = end_t - app.playback_time;

            let y_bottom = key_y - (time_until_hit * app.note_speed_px_per_sec as f64) as f32;
            let y_top = key_y - (time_until_end * app.note_speed_px_per_sec as f64) as f32;

            let y = y_top;
            let height = (y_bottom - y_top).max(3.0);

            if y > screen_h || y + height < 0.0 { continue; }

            let color = get_channel_color(note.channel, note.velocity, alpha);
            draw_rectangle(x, y, note_width, height, color);
            
            let border_color = Color::new(color.r * 0.6, color.g * 0.6, color.b * 0.6, alpha);
            draw_rectangle_lines(x, y, note_width, height, 1.0, border_color);

            let cap_color = Color::new((color.r * 1.5).min(1.0), (color.g * 1.5).min(1.0), (color.b * 1.5).min(1.0), alpha);
            draw_rectangle(x, y + height - 2.0, note_width, 2.0, cap_color);

            if app.show_velocity { 
                draw_velocity_text(note.velocity, center_x, y + height - 3.0);
            }
        }
    }
}

fn draw_piano_keys(app: &PianoRollApp, screen_h: f32, piano_x_start: f32, white_key_width: f32, black_key_width: f32) {
    for i in 21..=108 {
        let (is_black, white_idx) = get_key_pos(i);
        if !is_black {
            let color = get_active_key_color(app, i).unwrap_or(BLACK);
            let x = piano_x_start + white_idx * white_key_width;
            
            draw_rectangle(x, screen_h - KEY_HEIGHT, white_key_width, KEY_HEIGHT, color);
            draw_rectangle_lines(x, screen_h - KEY_HEIGHT, white_key_width, KEY_HEIGHT, 1.0, Color::new(0.2, 0.2, 0.2, 1.0));
        }
    }

    for i in 21..=108 {
        let (is_black, white_idx) = get_key_pos(i);
        if is_black {
            let color = get_active_key_color(app, i).unwrap_or(Color::new(0.1, 0.1, 0.1, 1.0));
            let center_x = piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);
            let x = center_x - (black_key_width / 2.0);
            
            draw_rectangle(x, screen_h - KEY_HEIGHT + 1.0, black_key_width, KEY_HEIGHT * 0.65, color);
        }
    }
}

fn draw_drum_pads(app: &PianoRollApp, screen_h: f32, drum_highway_w: f32, drum_x_start: f32) {
    let lane_w = drum_highway_w / 8.0;
    for lane in 0..8 {
        let x = drum_x_start + (lane as f32 * lane_w);

        let max_vel = app.active_pitches[DRUM_CHANNEL as usize]
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

fn draw_hud(app: &PianoRollApp, screen_w: f32) {
    let time_str = format!("Time: {:.1}s ({:.2}x)", app.playback_time, app.playback_speed);
    let time_size = measure_text(&time_str, None, 24, 1.0);
    let time_color = if app.practice_mode { ORANGE } else if app.is_playing { GREEN } else { YELLOW };
    draw_text(&time_str, (screen_w / 2.0) - (time_size.width / 2.0), 30.0, 24.0, time_color);

    let mut cc_text_y = 30.0;
    let cc_text_x = screen_w - 280.0;
    let ccs_active = app.cc_values.iter().any(|ch_array| ch_array.iter().any(|v| v.is_some()));

    if app.show_cc && ccs_active {
        draw_text("MIDI CC Monitor", cc_text_x, cc_text_y, 20.0, WHITE);
        cc_text_y += 25.0;

        for ch in 0..NUM_CHANNELS {
            for (controller, value_opt) in app.cc_values[ch].iter().enumerate() {
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

    if app.show_legend {
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

fn draw_velocity_text(velocity: u8, anchor_x: f32, anchor_y: f32) {
    let vel_text = format!("{}", velocity);
    let text_size = measure_text(&vel_text, None, 14, 1.0);
    let text_x = anchor_x - (text_size.width / 2.0);
    
    draw_text(&vel_text, text_x + 1.0, anchor_y + 1.0, 14.0, BLACK);
    draw_text(&vel_text, text_x, anchor_y, 14.0, WHITE);
}

fn get_active_key_color(app: &PianoRollApp, pitch: u8) -> Option<Color> {
    for ch in 0..NUM_CHANNELS {
        if ch as u8 == DRUM_CHANNEL { continue; }
        let (vel, count) = app.active_pitches[ch][pitch as usize];
        if count > 0 {
            return Some(get_channel_color(ch as u8, vel, 1.0));
        }
    }
    None
}
