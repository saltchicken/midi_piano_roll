use macroquad::prelude::*;
use midir::{Ignore, MidiInput, MidiInputConnection};
use std::sync::mpsc;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
enum MidiMessage {
    NoteOn { pitch: u8, velocity: u8, timestamp: f64 },
    NoteOff { pitch: u8, timestamp: f64 },
}

struct NoteInfo {
    pitch: u8,
    velocity: u8, 
    start_time: f64,
    end_time: Option<f64>,
}

fn setup_midi(tx: mpsc::Sender<MidiMessage>, app_start: Instant) -> Result<MidiInputConnection<()>, Box<dyn std::error::Error>> {
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
    
    let in_port = in_ports.last().unwrap();
    println!("Connecting to: {}", midi_in.port_name(in_port)?);

    let conn = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            // Timestamp the event precisely when it arrives in the background thread
            let timestamp = app_start.elapsed().as_secs_f64();
            
            if message.len() >= 3 {
                let status = message[0] & 0xF0;
                let pitch = message[1];
                let velocity = message[2];

                if status == 0x90 { // Note On
                    if velocity > 0 {
                        let _ = tx.send(MidiMessage::NoteOn { pitch, velocity, timestamp });
                    } else {
                        let _ = tx.send(MidiMessage::NoteOff { pitch, timestamp });
                    }
                } else if status == 0x80 { // Note Off
                    let _ = tx.send(MidiMessage::NoteOff { pitch, timestamp });
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
        (false, 0.0), (true, 0.5), (false, 1.0), (true, 1.5),
        (false, 2.0), (false, 3.0), (true, 3.5), (false, 4.0),
        (true, 4.5), (false, 5.0), (true, 5.5), (false, 6.0),
    ];
    
    let note_in_octave = pitch % 12;
    let octave = (pitch / 12) as f32;
    let (is_black, rel_pos) = notes_in_octave[note_in_octave as usize];
    
    let absolute_white_idx = octave * 7.0 + rel_pos;
    let adjusted_idx = absolute_white_idx - 12.0;
    
    (is_black, adjusted_idx)
}

#[macroquad::main("MIDI Piano Roll")]
async fn main() {
    // Standardize timekeeping using std::time::Instant
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
    let mut active_pitches = [false; 128];
    let note_speed = 300.0;

    loop {
        clear_background(Color::new(0.1, 0.1, 0.12, 1.0));
        
        // Sync render time to the shared clock
        let current_time = app_start.elapsed().as_secs_f64();

        while let Ok(msg) = rx.try_recv() {
            match msg {
                MidiMessage::NoteOn { pitch, velocity, timestamp } => {
                    notes.push(NoteInfo {
                        pitch,
                        velocity,
                        start_time: timestamp, // Use thread timestamp
                        end_time: None,
                    });
                    active_pitches[pitch as usize] = true;
                }
                MidiMessage::NoteOff { pitch, timestamp } => {
                    active_pitches[pitch as usize] = false;
                    if let Some(note) = notes.iter_mut().rev().find(|n| n.pitch == pitch && n.end_time.is_none()) {
                        note.end_time = Some(timestamp); // Use thread timestamp
                    }
                }
            }
        }

        let screen_w = screen_width();
        let screen_h = screen_height();
        let key_height = 80.0;
        
        let num_white_keys = 52.0;
        let white_key_width = screen_w / num_white_keys;
        let black_key_width = white_key_width * 0.6;

        for note in &notes {
            let (is_black, white_idx) = get_key_pos(note.pitch);
            let center_x = white_idx * white_key_width + (white_key_width / 2.0);
            
            let note_width = if is_black { black_key_width } else { white_key_width - 2.0 };
            let x = center_x - (note_width / 2.0);

            let end_t = note.end_time.unwrap_or(current_time);
            
            let y_bottom = screen_h - key_height - ((current_time - end_t) * note_speed as f64) as f32;
            let y_top = screen_h - key_height - ((current_time - note.start_time) * note_speed as f64) as f32;

            let y = y_top;
            let height = (y_bottom - y_top).max(3.0);

            if y > screen_h || y + height < 0.0 {
                continue;
            }

            let velocity_alpha = (note.velocity as f32 / 127.0).clamp(0.3, 1.0);

            let color = if note.end_time.is_none() {
                Color::new(0.0, 0.8, 1.0, 0.9 * velocity_alpha)
            } else {
                Color::new(0.0, 0.5, 0.8, 0.7 * velocity_alpha)
            };

            draw_rectangle(x, y, note_width, height, color);
        }

        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if !is_black {
                let x = white_idx * white_key_width;
                let color = if active_pitches[i as usize] { Color::new(0.7, 0.9, 1.0, 1.0) } else { WHITE };
                draw_rectangle(x, screen_h - key_height, white_key_width, key_height, color);
                draw_rectangle_lines(x, screen_h - key_height, white_key_width, key_height, 1.0, GRAY);
            }
        }

        for i in 21..=108 {
            let (is_black, white_idx) = get_key_pos(i);
            if is_black {
                let center_x = white_idx * white_key_width + (white_key_width / 2.0);
                let x = center_x - (black_key_width / 2.0);
                let color = if active_pitches[i as usize] { Color::new(0.3, 0.5, 0.7, 1.0) } else { BLACK };
                draw_rectangle(x, screen_h - key_height, black_key_width, key_height * 0.65, color);
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
