mod music_controller;
mod library_controller;

use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use music_controller::MusicController;

pub struct AppState {
    player: Arc<Mutex<Option<MusicController>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            player: Arc::new(Mutex::new(None)),
        })
        .setup(|app| {
            let state = app.state::<AppState>();
            let mut slot = state.player.lock().unwrap();

            let (player, rx) = MusicController::new(app.handle().clone()).expect("Failed to create music controller");

            *slot = Some(player);
            drop(slot);

            MusicController::create_queue_thread(state.player.clone(), rx);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            toggle_playback,
            scan_dir,
            load_song,
            skip_forward,
            skip_backward,
            set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn toggle_playback(state: State<'_, AppState>) -> Result<bool, String> {
    let mut guard = state
        .player
        .lock()
        .map_err(|e| e.to_string())?;
    let player = guard
        .as_mut()
        .ok_or_else(|| "Player not initialised".to_string())?;
    player.toggle_playback();
    // return the new paused state so the frontend can update its icon
    Ok(player.parameters.is_paused)
}

#[tauri::command]
fn load_song(state: State<'_, AppState>, dir: String) -> Result<String, String> {
    let mut guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = guard.as_mut().ok_or_else(|| "Player not initialised".to_string())?;
    player.add_to_queue(dir);
    Ok("Added song to queue".to_string())
}

#[tauri::command]
fn skip_forward(state: State<'_, AppState>) -> Result<String, String> {
    let mut guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = guard.as_mut().ok_or_else(|| "Player not initialised".to_string())?;
    player.skip_forward();
    Ok("Skipped forward".to_string())
}

#[tauri::command]
fn skip_backward(state: State<'_, AppState>) -> Result<String, String> {
    let mut guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = guard.as_mut().ok_or_else(|| "Player not initialised".to_string())?;
    player.skip_backward();
    Ok("Skipped backward".to_string())
}

#[tauri::command]
fn scan_dir(dir_str: String) -> Result<Vec<Vec<String>>, String> {
    library_controller::scan_dir(dir_str).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    let mut guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = guard.as_mut().ok_or_else(|| "Player not initialised".to_string())?;
    player.set_volume(volume);
    Ok(())
}