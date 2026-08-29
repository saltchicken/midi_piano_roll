use macroquad::prelude::*;
use midir::{Ignore, MidiInput, MidiInputConnection};
use std::sync::mpsc;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
enum MidiMessage {
    NoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
        timestamp: f64,
    },
    NoteOff {
        channel: u8,
        pitch: u8,
        timestamp: f64,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
        timestamp: f64,
    },
}

struct NoteInfo {
    channel: u8,
    pitch: u8,
    velocity: u8,
    start_time: f64,
    end_time: Option<f64>,
}

fn setup_midi(
    tx: mpsc::Sender<MidiMessage>,
    app_start: Instant,
) -> Result<MidiInputConnection<()>, Box<dyn std::error::Error>> {
    let mut midi_in = MidiInput::new("Piano Roll Input")?;
    midi_in.ignore(Ignore::None);

    let in_ports = midi_in.ports();
    if in_ports.is_empty() {
        return Err("No MIDI input ports found.".into());
    }

    println!("Available MIDI ports:");
    for (i, p) in in_ports.iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }

    // Search for "Midi Through", fallback to the first port if it isn't found
    let in_port = in_ports
        .iter()
        .find(|p| {
            midi_in
                .port_name(p)
                .unwrap_or_default()
                .contains("Midi Through")
        })
        .unwrap_or_else(|| in_ports.first().unwrap());

    println!("Connecting to: {}", midi_in.port_name(in_port)?);

    let conn = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            let timestamp = app_start.elapsed().as_secs_f64();

            if message.len() >= 3 {
                // Mask out the channel to get just the message type
                let status = message[0] & 0xF0;
                // Extract the channel (lower 4 bits)
                let channel = message[0] & 0x0F;

                let data1 = message[1];
                let data2 = message[2];

                if status == 0x90 {
                    // Note On
                    if data2 > 0 {
                        let _ = tx.send(MidiMessage::NoteOn {
                            channel,
                            pitch: data1,
                            velocity: data2,
                            timestamp,
                        });
                    } else {
                        let _ = tx.send(MidiMessage::NoteOff {
                            channel,
                            pitch: data1,
                            timestamp,
                        });
                    }
                } else if status == 0x80 {
                    // Note Off
                    let _ = tx.send(MidiMessage::NoteOff {
                        channel,
                        pitch: data1,
                        timestamp,
                    });
                } else if status == 0xB0 {
                    // Control Change (CC)
                    let _ = tx.send(MidiMessage::ControlChange {
                        channel,
                        controller: data1,
                        value: data2,
                        timestamp,
                    });
                }
            }
        },
        (),
    )?;

    Ok(conn)
}

fn get_key_pos(pitch: u8) -> (bool, f32) {
    if pitch < 21 || pitch > 108 {
        return (false, 0.0);
    }

    let notes_in_octave = [
        (false, 0.0),
        (true, 0.5),
        (false, 1.0),
        (true, 1.5),
        (false, 2.0),
        (false, 3.0),
        (true, 3.5),
        (false, 4.0),
        (true, 4.5),
        (false, 5.0),
        (true, 5.5),
        (false, 6.0),
    ];

    let note_in_octave = pitch % 12;
    let octave = (pitch / 12) as f32;
    let (is_black, rel_pos) = notes_in_octave[note_in_octave as usize];

    let absolute_white_idx = octave * 7.0 + rel_pos;
    let adjusted_idx = absolute_white_idx - 12.0;

    (is_black, adjusted_idx)
}

fn get_channel_color(channel: u8, alpha: f32) -> Color {
    let colors = [
        (1.0, 0.3, 0.3),
        (0.3, 1.0, 0.3),
        (0.3, 0.5, 1.0),
        (1.0, 1.0, 0.3), // CH 1-4
        (1.0, 0.6, 0.2),
        (0.8, 0.3, 0.8),
        (0.3, 1.0, 1.0),
        (1.0, 0.4, 0.7), // CH 5-8
        (0.6, 0.8, 0.2),
        (0.4, 0.8, 1.0),
        (0.9, 0.2, 0.5),
        (0.2, 0.8, 0.6), // CH 9-12
        (0.7, 0.4, 0.0),
        (0.6, 0.6, 0.6),
        (0.8, 0.8, 0.9),
        (1.0, 0.8, 0.6), // CH 13-16
    ];
    let (r, g, b) = colors[(channel as usize) % 16];
    Color::new(r, g, b, alpha)
}

// Groups standard GM drum pitches into 8 visual lanes
fn get_drum_lane(pitch: u8) -> Option<(&'static str, usize)> {
    match pitch {
        35 | 36 => Some(("KICK", 0)),
        38 | 40 => Some(("SNR", 1)),
        42 | 44 => Some(("CHH", 2)), // Closed Hi-hat
        46 => Some(("OHH", 3)),      // Open Hi-hat
        41 | 43 | 45 | 47 | 48 | 50 => Some(("TOM", 4)),
        49 | 52 | 55 | 57 => Some(("CRSH", 5)),
        51 | 53 | 59 => Some(("RIDE", 6)),
        _ => Some(("PERC", 7)), // Catch-all for other percussions
    }
}

#[macroquad::main("MIDI Piano Roll")]
async fn main() {
    let app_start = Instant::now();
    let (tx, rx) = mpsc::channel();

    let _midi_conn = match setup_midi(tx, app_start) {
        Ok(conn) => Some(conn),
        Err(e) => {
            eprintln!("Failed to setup MIDI: {}", e);
            None
        }
    };

    let mut notes: Vec<NoteInfo> = Vec::new();
    let mut active_pitches = [[false; 128]; 16];

    // Track CC values and their last updated timestamp for 16 channels, each with 128 controllers
    let mut cc_values: [[Option<(u8, f64)>; 128]; 16] = [[None; 128]; 16];
    let note_speed = 300.0;
    let cc_timeout = 3.0; // Seconds before a CC value disappears from the HUD

    // State for toggling UI elements
    let mut show_drums = true;
    let mut show_cc = true;
    let mut show_hints = false;
    let mut show_legend = false;

    loop {
        // Toggle states
        if is_key_pressed(KeyCode::D) {
            show_drums = !show_drums;
        }
        if is_key_pressed(KeyCode::C) {
            show_cc = !show_cc;
        }
        if is_key_pressed(KeyCode::L) {
            show_legend = !show_legend;
        }
        // Slash key typically represents both '/' and '?' on standard US keyboards
        if is_key_pressed(KeyCode::Slash) {
            show_hints = !show_hints;
        }

        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));

        let current_time = app_start.elapsed().as_secs_f64();

        while let Ok(msg) = rx.try_recv() {
            match msg {
                MidiMessage::NoteOn {
                    channel,
                    pitch,
                    velocity,
                    timestamp,
                } => {
                    notes.push(NoteInfo {
                        channel,
                        pitch,
                        velocity,
                        start_time: timestamp,
                        end_time: None,
                    });
                    active_pitches[channel as usize][pitch as usize] = true;
                }
                MidiMessage::NoteOff {
                    channel,
                    pitch,
                    timestamp,
                } => {
                    active_pitches[channel as usize][pitch as usize] = false;
                    if let Some(note) = notes
                        .iter_mut()
                        .rev()
                        .find(|n| n.pitch == pitch && n.channel == channel && n.end_time.is_none())
                    {
                        note.end_time = Some(timestamp);
                    }
                }
                MidiMessage::ControlChange {
                    channel,
                    controller,
                    value,
                    timestamp,
                } => {
                    cc_values[channel as usize][controller as usize] = Some((value, timestamp));
                }
            }
        }

        // Clean up expired CC values
        for ch in 0..16 {
            for v in cc_values[ch].iter_mut() {
                if let Some((_, ts)) = v {
                    if current_time - *ts > cc_timeout {
                        *v = None;
                    }
                }
            }
        }

        let screen_w = screen_width();
        let screen_h = screen_height();
        let key_height = 80.0;

        // Reserve up to 300 pixels on the left for drums, conditionally
        let drum_highway_w = if show_drums {
            300.0_f32.min(screen_w * 0.3)
        } else {
            0.0
        };
        let piano_w = screen_w - drum_highway_w;

        let drum_x_start = 0.0;
        let piano_x_start = drum_highway_w;

        let num_white_keys = 52.0;
        // Scale white keys based on piano_w, not screen_w
        let white_key_width = piano_w / num_white_keys;
        let black_key_width = white_key_width * 0.6;

        // Render falling notes
        for note in &notes {
            if note.channel == 9 {
                // DRUM RENDERING (On the Left)
                if show_drums {
                    if let Some((_, lane)) = get_drum_lane(note.pitch) {
                        let lane_w = drum_highway_w / 8.0;
                        let center_x = drum_x_start + (lane as f32 * lane_w) + (lane_w / 2.0);

                        // Drums are instantaneous; calculate Y based solely on start_time
                        let y = screen_h
                            - key_height
                            - ((current_time - note.start_time) * note_speed as f64) as f32;

                        if y > screen_h || y < -50.0 {
                            continue;
                        }

                        let velocity_alpha = (note.velocity as f32 / 127.0).clamp(0.4, 1.0);
                        let color = get_channel_color(note.channel, velocity_alpha);

                        draw_circle(center_x, y, lane_w * 0.3, color);
                    }
                }
            } else {
                // STANDARD PIANO RENDERING (Offset by drum width)
                let (is_black, white_idx) = get_key_pos(note.pitch);
                let center_x =
                    piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);

                let note_width = if is_black {
                    black_key_width
                } else {
                    white_key_width - 2.0
                };
                let x = center_x - (note_width / 2.0);

                let end_t = note.end_time.unwrap_or(current_time);
                let y_bottom =
                    screen_h - key_height - ((current_time - end_t) * note_speed as f64) as f32;
                let y_top = screen_h
                    - key_height
                    - ((current_time - note.start_time) * note_speed as f64) as f32;

                let y = y_top;
                let height = (y_bottom - y_top).max(3.0);

                if y > screen_h || y + height < 0.0 {
                    continue;
                }

                let velocity_alpha = (note.velocity as f32 / 127.0).clamp(0.3, 1.0);
                let base_alpha = if note.end_time.is_none() { 0.9 } else { 0.5 };
                let color = get_channel_color(note.channel, base_alpha * velocity_alpha);

                // Draw the main note body
                draw_rectangle(x, y, note_width, height, color);

                // 1. Draw a subtle darker border so consecutive notes don't visually merge
                let border_color = Color::new(color.r * 0.6, color.g * 0.6, color.b * 0.6, color.a);
                draw_rectangle_lines(x, y, note_width, height, 1.0, border_color);

                // 2. Draw a brighter "cap" at the start of the note (the bottom edge)
                // This creates a visual "strike" making repeated fast notes obvious
                let cap_color = Color::new(
                    (color.r * 1.5).min(1.0),
                    (color.g * 1.5).min(1.0),
                    (color.b * 1.5).min(1.0),
                    color.a,
                );
                // The bottom of the falling note is y + height
                draw_rectangle(x, y + height - 2.0, note_width, 2.0, cap_color);
            }
        }

        // Render piano keys (white keys)
        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if !is_black {
                let mut active_channel = None;
                for ch in 0..16 {
                    // Ignore drum channel for the piano roll keys entirely
                    if ch == 9 {
                        continue;
                    }

                    if active_pitches[ch][i as usize] {
                        active_channel = Some(ch as u8);
                        break;
                    }
                }

                let x = piano_x_start + white_idx * white_key_width;
                let color = if let Some(ch) = active_channel {
                    get_channel_color(ch, 1.0)
                } else {
                    BLACK
                };
                draw_rectangle(x, screen_h - key_height, white_key_width, key_height, color);
                draw_rectangle_lines(
                    x,
                    screen_h - key_height,
                    white_key_width,
                    key_height,
                    1.0,
                    GRAY,
                );
            }
        }

        // Render piano keys (black keys)
        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if is_black {
                let mut active_channel = None;
                for ch in 0..16 {
                    // Ignore drum channel for the piano roll keys entirely
                    if ch == 9 {
                        continue;
                    }

                    if active_pitches[ch][i as usize] {
                        active_channel = Some(ch as u8);
                        break;
                    }
                }

                let center_x =
                    piano_x_start + white_idx * white_key_width + (white_key_width / 2.0);
                let x = center_x - (black_key_width / 2.0);
                let color = if let Some(ch) = active_channel {
                    get_channel_color(ch, 1.0)
                } else {
                    Color::new(0.1, 0.1, 0.1, 1.0)
                };
                draw_rectangle(
                    x,
                    screen_h - key_height + 1.0,
                    black_key_width,
                    key_height * 0.65,
                    color,
                );
            }
        }

        // Render Drum Pads
        if show_drums {
            let lane_w = drum_highway_w / 8.0;
            for lane in 0..8 {
                let x = drum_x_start + (lane as f32 * lane_w);

                // Check if any pitch mapped to this lane is currently active
                let is_active = active_pitches[9].iter().enumerate().any(|(p, &active)| {
                    active && get_drum_lane(p as u8).map(|(_, l)| l) == Some(lane)
                });

                let color = if is_active {
                    get_channel_color(9, 1.0)
                } else {
                    Color::new(0.2, 0.2, 0.2, 1.0)
                };

                // Draw Pad
                draw_rectangle(x, screen_h - key_height, lane_w - 2.0, key_height, color);
                draw_rectangle_lines(
                    x,
                    screen_h - key_height,
                    lane_w - 2.0,
                    key_height,
                    1.0,
                    GRAY,
                );

                // Draw Label
                let label = match lane {
                    0 => "KICK",
                    1 => "SNR",
                    2 => "CHH",
                    3 => "OHH",
                    4 => "TOM",
                    5 => "CRSH",
                    6 => "RIDE",
                    _ => "PERC",
                };

                // Center the text roughly in the pad
                let text_size = measure_text(label, None, 16u16, 1.0);
                let text_x = x + (lane_w / 2.0) - (text_size.width / 2.0);
                draw_text(
                    label,
                    text_x,
                    screen_h - (key_height / 2.0) + (text_size.height / 2.0),
                    16.0,
                    WHITE,
                );
            }
        }

        // Render CC Monitor HUD in top left (offset by drum highway if open)
        let mut cc_text_y = if show_hints { 150.0 } else { 30.0 };
        let cc_text_x = screen_w - 280.0; // Anchored to the right

        let ccs_active = cc_values
            .iter()
            .any(|ch_array| ch_array.iter().any(|v| v.is_some()));

        if show_cc && ccs_active {
            draw_text("MIDI CC Monitor", cc_text_x, cc_text_y, 20.0, WHITE);
            cc_text_y += 25.0;

            for ch in 0..16 {
                for (controller, value_opt) in cc_values[ch].iter().enumerate() {
                    if let Some((value, _)) = *value_opt {
                        // Channel numbering conceptually ranges 1-16 to the user (ch + 1)
                        let text = format!("CH {:02} | CC {:03}: {:03}", ch + 1, controller, value);
                        draw_text(&text, cc_text_x, cc_text_y, 16.0, LIGHTGRAY);

                        // Draw small bar graph indicating the level (0-127)
                        let bar_width = 100.0;
                        let fill_width = (value as f32 / 127.0) * bar_width;
                        let bar_x = cc_text_x + 145.0; // Pushed right relative to the text block

                        // Background bar
                        draw_rectangle(
                            bar_x,
                            cc_text_y - 12.0,
                            bar_width,
                            10.0,
                            Color::new(0.2, 0.2, 0.2, 0.8),
                        );
                        // Foreground (fill) bar
                        draw_rectangle(
                            bar_x,
                            cc_text_y - 12.0,
                            fill_width,
                            10.0,
                            Color::new(0.0, 0.8, 0.5, 0.8),
                        );

                        cc_text_y += 20.0;
                    }
                }
            }
        }

        // Render Hotkey Hints in top right
        if show_hints {
            let hint_toggle = "[?] Toggle Hints";
            let hint_drums = format!("[D] Drums: {}", if show_drums { "ON" } else { "OFF" });
            let hint_cc = format!("[C] CC Monitor: {}", if show_cc { "ON" } else { "OFF" });
            let hint_legend = format!("[L] Legend: {}", if show_legend { "ON" } else { "OFF" });

            // Measure the widest text line so we can right-align it properly
            let m1 = measure_text(hint_toggle, None, 20, 1.0);
            let m2 = measure_text(&hint_drums, None, 20, 1.0);
            let m3 = measure_text(&hint_cc, None, 20, 1.0);
            let m4 = measure_text(&hint_legend, None, 20, 1.0);

            let max_w = m1.width.max(m2.width).max(m3.width).max(m4.width);

            let hints_x = screen_w - max_w - 15.0;
            let mut hints_y = 30.0;

            draw_text(hint_toggle, hints_x, hints_y, 20.0, WHITE);
            hints_y += 25.0;
            draw_text(&hint_drums, hints_x, hints_y, 20.0, WHITE);
            hints_y += 25.0;
            draw_text(&hint_cc, hints_x, hints_y, 20.0, WHITE);
            hints_y += 25.0;
            draw_text(&hint_legend, hints_x, hints_y, 20.0, WHITE);
        }

        // Render Channel Color Legend
        if show_legend {
            let legend_x = 20.0;
            let mut legend_y = 30.0;

            // Draw a semi-transparent background to ensure readability over notes
            draw_rectangle(
                legend_x - 10.0, 
                legend_y - 20.0, 
                150.0, 
                390.0, 
                Color::new(0.0, 0.0, 0.0, 0.6)
            );

            draw_text("Channels", legend_x, legend_y, 20.0, WHITE);
            legend_y += 15.0;

            for ch in 0..16 {
                let color = get_channel_color(ch as u8, 1.0);
                
                // Draw color swatch
                draw_rectangle(legend_x, legend_y, 16.0, 16.0, color);
                draw_rectangle_lines(legend_x, legend_y, 16.0, 16.0, 1.0, GRAY);
                
                // Draw channel label (explicitly label channel 10 as drums)
                let label = if ch == 9 {
                    format!("CH 10 (Drums)")
                } else {
                    format!("CH {}", ch + 1)
                };
                
                draw_text(&label, legend_x + 25.0, legend_y + 13.0, 16.0, WHITE);
                
                legend_y += 22.0;
            }
        }

        notes.retain(|n| {
            if let Some(et) = n.end_time {
                ((current_time - et) * note_speed as f64) < screen_h as f64
            } else {
                true
            }
        });

        next_frame().await
    }
}
