mod music_controller;
use std::sync::Mutex;
use tauri::Manager;
use music_controller::MusicController;

pub struct AppState {
    player: Mutex<Option<MusicController>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            player: Mutex::new(None),
        })
        .setup(|app| {
            let state = app.state::<AppState>();
            let mut slot = state.player.lock().unwrap();

            let mut player = MusicController::new().expect("Failed to create music controller");
            player.add_to_queue(
                "/Users/mael/RustroverProjects/music-visualizer/music/Gorillaz - Feel Good Inc..flac"
                    .to_string(),
            );
            player.start_song();
            player.add_to_queue(
                "/Users/mael/RustroverProjects/music-visualizer/music/Gorillaz - Feel Good Inc..flac"
                    .to_string(),
            );
            player.start_song();

            *slot = Some(player);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}