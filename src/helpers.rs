use macroquad::prelude::*;

pub fn get_key_pos(pitch: u8) -> (bool, f32) {
    if pitch < 21 || pitch > 108 { return (false, 0.0); }

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

pub fn get_channel_color(channel: u8, velocity: u8, alpha: f32) -> Color {
    let colors = [
        (1.0, 0.3, 0.3), (0.3, 1.0, 0.3), (0.3, 0.5, 1.0), (1.0, 1.0, 0.3), 
        (1.0, 0.6, 0.2), (0.8, 0.3, 0.8), (0.3, 1.0, 1.0), (1.0, 0.4, 0.7), 
        (0.6, 0.8, 0.2), (0.4, 0.8, 1.0), (0.9, 0.2, 0.5), (0.2, 0.8, 0.6), 
        (0.7, 0.4, 0.0), (0.6, 0.6, 0.6), (0.8, 0.8, 0.9), (1.0, 0.8, 0.6), 
    ];
    let (r, g, b) = colors[(channel as usize) % 16];
    let intensity = (velocity as f32 / 127.0).clamp(0.2, 1.0);
    Color::new(r * intensity, g * intensity, b * intensity, alpha)
}

pub fn get_drum_lane(pitch: u8) -> Option<(&'static str, usize)> {
    match pitch {
        35 | 36 => Some(("KICK", 0)),
        38 | 40 => Some(("SNR", 1)),
        42 | 44 => Some(("CHH", 2)), 
        46 => Some(("OHH", 3)),      
        41 | 43 | 45 | 47 | 48 | 50 => Some(("TOM", 4)),
        49 | 52 | 55 | 57 => Some(("CRSH", 5)),
        51 | 53 | 59 => Some(("RIDE", 6)),
        _ => Some(("PERC", 7)), 
    }
}
