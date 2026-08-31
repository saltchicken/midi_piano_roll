mod app;
mod constants;
mod helpers;
mod midi;
mod types;

use macroquad::prelude::*;
use std::sync::mpsc;
use std::time::Instant;

use app::PianoRollApp;
use midi::setup_midi;

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

    let mut app = PianoRollApp::new();

    loop {
        let current_time = app_start.elapsed().as_secs_f64();
        
        app.update(&rx, current_time);
        app.draw(current_time);

        next_frame().await
    }
}
