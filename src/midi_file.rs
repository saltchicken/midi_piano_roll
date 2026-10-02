use midly::{MetaMessage, Smf, Timing, TrackEventKind};
use crate::types::NoteInfo;

pub fn load_midi_file(path: &str) -> Result<Vec<NoteInfo>, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let smf = Smf::parse(&data)?;

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(ticks) => f64::from(u16::from(ticks)),
        Timing::Timecode(_, _) => return Err("SMPTE timecode not yet supported".into()),
    };

    let mut all_notes = Vec::new();
    
    // Default tempo is 120 BPM (500,000 microseconds per beat)
    let mut tempo_mpq = 500_000.0; 
    
    for (_track_idx, track) in smf.tracks.iter().enumerate() {
        let mut current_time_sec = 0.0;
        let mut active_notes: [Option<(f64, u8)>; 128] = [None; 128]; // (start_time, velocity)

        for event in track {
            let delta = u32::from(event.delta) as u64;
            
            // Convert ticks to seconds based on current tempo
            let delta_sec = (delta as f64 / ticks_per_beat) * (tempo_mpq / 1_000_000.0);
            current_time_sec += delta_sec;

            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                    tempo_mpq = u32::from(t) as f64;
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = u8::from(channel);
                    match message {
                        midly::MidiMessage::NoteOn { key, vel } => {
                            let k = u8::from(key);
                            let v = u8::from(vel);
                            if v > 0 {
                                active_notes[k as usize] = Some((current_time_sec, v));
                            } else {
                                // Velocity 0 functions as NoteOff
                                if let Some((start_time, start_vel)) = active_notes[k as usize].take() {
                                    all_notes.push(NoteInfo {
                                        channel: ch,
                                        pitch: k,
                                        velocity: start_vel,
                                        start_time,
                                        end_time: Some(current_time_sec),
                                        is_hit: false,
                                    });
                                }
                            }
                        }
                        midly::MidiMessage::NoteOff { key, .. } => {
                            let k = u8::from(key);
                            if let Some((start_time, start_vel)) = active_notes[k as usize].take() {
                                all_notes.push(NoteInfo {
                                    channel: ch,
                                    pitch: k,
                                    velocity: start_vel,
                                    start_time,
                                    end_time: Some(current_time_sec),
                                    is_hit: false,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    // Sort chronologically so they render consistently
    all_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    Ok(all_notes)
}
