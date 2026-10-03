use egui_macroquad::egui;
use crate::app::PianoRollApp;

pub fn draw_controls(app: &mut PianoRollApp, ctx: &egui::Context) {
    egui::Window::new("Controls")
        .default_pos([20.0, 20.0])
        .show(ctx, |ui| {
            ui.heading("Playback");
            
            ui.horizontal(|ui| {
                if ui.button(if app.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    app.is_playing = !app.is_playing;
                }
                if ui.button("📂 Load MIDI").clicked() {
                    app.load_song();
                }
                if ui.button("⏹ Stop & Clear").clicked() {
                    app.active_pitches = [[(0u8, 0u8); crate::constants::NUM_PITCHES]; crate::constants::NUM_CHANNELS];
                    app.song_notes.clear();
                    app.playback_time = 0.0;
                    app.is_playing = false;
                }
            });

            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui.button("⏪").clicked() { app.playback_speed -= 0.25; }
                if ui.button("1x").clicked() { app.playback_speed = 1.0; }
                if ui.button("⏩").clicked() { app.playback_speed += 0.25; }
                if ui.button("Reverse").clicked() { app.playback_speed *= -1.0; }
            });
            
            ui.label(format!("Current Speed: {:.2}x", app.playback_speed));
            ui.label(format!("Time: {:.1}s", app.playback_time));

            ui.add_space(15.0);
            ui.heading("Settings");
            
            ui.checkbox(&mut app.practice_mode, "Practice Mode");
            ui.checkbox(&mut app.show_drums, "Show Drum Highway");
            ui.checkbox(&mut app.show_cc, "Show CC Monitor");
            ui.checkbox(&mut app.show_legend, "Show Channel Legend");
            ui.checkbox(&mut app.show_velocity, "Show Velocity Numbers");
            
            ui.add_space(10.0);
            ui.label("Zoom (Note Speed)");
            ui.add(egui::Slider::new(&mut app.note_speed_px_per_sec, 50.0..=2000.0));

            ui.add_space(10.0);
            ui.label("Piano Range");
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.add(egui::DragValue::new(&mut app.min_pitch).range(0..=127));
                ui.label("Max:");
                ui.add(egui::DragValue::new(&mut app.max_pitch).range(0..=127));
            });
            
            // Prevent inverted ranges
            if app.min_pitch > app.max_pitch {
                app.max_pitch = app.min_pitch;
            }
        });
}
