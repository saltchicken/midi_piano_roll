mod app;
mod constants;
mod helpers;
mod midi;
mod midi_file;
mod types;

use macroquad::prelude::*;
use std::sync::mpsc;

use app::PianoRollApp;
use midi::setup_midi;

#[macroquad::main("MIDI Piano Roll")]
async fn main() {
    let (tx, rx) = mpsc::channel();

    let _midi_conn = match setup_midi(tx) {
        Ok(conn) => Some(conn),
        Err(e) => {
            eprintln!("Failed to setup MIDI: {}", e);
            None
        }
    };

    let mut app = PianoRollApp::new();

    loop {
        let dt = (get_frame_time() as f64).min(0.1);
        app.advance_time(dt);

        app.update(&rx);
        app.draw(); 

        egui_macroquad::ui(|egui_ctx| {
            app.ui(egui_ctx);
        });
        egui_macroquad::draw();

        next_frame().await
    }
}
