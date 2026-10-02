// src/midi.rs

use midir::{Ignore, MidiInput, MidiInputConnection};
use std::sync::mpsc;

use crate::types::MidiMessage;

pub fn setup_midi(
    tx: mpsc::Sender<MidiMessage>,
) -> Result<MidiInputConnection<()>, Box<dyn std::error::Error>> {
    let mut midi_in = MidiInput::new("Piano Roll Input")?;
    midi_in.ignore(Ignore::None);

    let in_ports = midi_in.ports();
    if in_ports.is_empty() { return Err("No MIDI input ports found.".into()); }

    println!("Available MIDI ports:");
    for (i, p) in in_ports.iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }

    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).unwrap_or_default().contains("Midi Through"))
        .unwrap_or_else(|| in_ports.first().unwrap());

    println!("Connecting to: {}", midi_in.port_name(in_port)?);

    let conn = midi_in.connect(
        in_port,
        "midir-read-input",
        move |_, message, _| {
            if message.len() >= 3 {
                let status = message[0] & 0xF0;
                let channel = message[0] & 0x0F;
                let data1 = message[1];
                let data2 = message[2];

                if status == 0x90 {
                    if data2 > 0 {
                        let _ = tx.send(MidiMessage::NoteOn { channel, pitch: data1, velocity: data2 });
                    } else {
                        let _ = tx.send(MidiMessage::NoteOff { channel, pitch: data1 });
                    }
                } else if status == 0x80 {
                    let _ = tx.send(MidiMessage::NoteOff { channel, pitch: data1 });
                } else if status == 0xB0 {
                    let _ = tx.send(MidiMessage::ControlChange { channel, controller: data1, value: data2 });
                }
            }
        },
        (),
    )?;

    Ok(conn)
}
