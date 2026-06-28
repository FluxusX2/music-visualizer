mod music_player;

use std::sync::Mutex;
use music_player::MusicPlayer;

pub struct AppState {
    player: Mutex<Option<MusicPlayer>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            player: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![play_sound, stop_sound, list_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn play_sound(state: tauri::State<AppState>) -> Result<String, String> {
    let file_path = r"C:\Users\maelb\Desktop\music\mrbright.wav";
    let mut p = state.player.lock().unwrap();

    match p.as_ref() {
        Some(player) => player
            .play()
            .map(|_| "Musik wird wiedergegeben".to_string())
            .map_err(|e| format!("Fehler beim Fortsetzen: {}", e)),
        None => match MusicPlayer::new(file_path) {
            Ok(player) => {
                *p = Some(player);
                Ok("Musik wird wiedergegeben".to_string())
            }
            Err(e) => Err(format!("Fehler beim Abspielen: {}", e)),
        }
    }
}

#[tauri::command]
fn stop_sound(state: tauri::State<AppState>) -> Result<String, String> {
    let p = state.player.lock().unwrap();

    match p.as_ref() {
        Some(player) => player
            .pause()
            .map(|_| "Musik pausiert".to_string())
            .map_err(|e| format!("Fehler beim Pausieren: {}", e)),
        None => Ok("Keine Musik aktiv".to_string()),
    }
}

#[tauri::command]
fn list_files(folder: String) -> Result<Vec<String>, String> {
    let path = std::path::Path::new(&folder);
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Ordner nicht lesbar: {}", e))?;

    let files: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            // Nur .wav Dateien (oder alle: entferne den Filter)
            if path.extension()?.to_str()? == "wav" {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}
