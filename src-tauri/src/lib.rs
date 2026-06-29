mod music_player;

use std::sync::Mutex;
use music_player::MusicPlayer;

pub struct AppState {
    player: Mutex<Option<MusicPlayer>>,
}

/// Stellt sicher, dass der Player initialisiert ist.
fn ensure_player(state: &tauri::State<AppState>) -> Result<(), String> {
    let mut p = state.player.lock().unwrap();
    if p.is_none() {
        *p = Some(MusicPlayer::new().map_err(|e| e.to_string())?);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            player: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            play_sound,
            stop_sound,
            list_files,
            add_to_queue,
            remove_from_queue,
            get_queue,
            tick,
            set_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[tauri::command]
fn play_sound(state: tauri::State<AppState>) -> Result<String, String> {
    ensure_player(&state)?;
    let mut p = state.player.lock().unwrap();
    p.as_mut()
        .unwrap()
        .play()
        .map(|_| "Musik wird wiedergegeben".to_string())
        .map_err(|e| format!("Fehler beim Fortsetzen: {}", e))
}

#[tauri::command]
fn stop_sound(state: tauri::State<AppState>) -> Result<String, String> {
    let mut p = state.player.lock().unwrap();
    match p.as_mut() {
        Some(player) => player
            .pause()
            .map(|_| "Musik pausiert".to_string())
            .map_err(|e| format!("Fehler beim Pausieren: {}", e)),
        None => Ok("Keine Musik aktiv".to_string()),
    }
}

/// Lists all Songs in the given folder with .wav or .flac extension.
#[tauri::command]
fn list_files(folder: String) -> Result<Vec<String>, String> {
    let path = std::path::Path::new(&folder);
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Ordner nicht lesbar: {}", e))?;

    let files: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let ext = path.extension()?.to_str()?.to_lowercase();
            if ext == "wav" || ext == "flac" {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}

/// Adds Song to the queue and plays the song if queue is empty.
#[tauri::command]
fn add_to_queue(state: tauri::State<AppState>, path: String) -> Result<String, String> {
    ensure_player(&state)?;
    let mut p = state.player.lock().unwrap();
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    p.as_mut()
        .unwrap()
        .enqueue(path)
        .map(|_| format!("Hinzugefügt: {}", name))
        .map_err(|e| format!("Fehler beim Hinzufügen: {}", e))
}

/// Removes Song from queue at specific index.
#[tauri::command]
fn remove_from_queue(state: tauri::State<AppState>, index: usize) -> Result<String, String> {
    let mut p = state.player.lock().unwrap();
    match p.as_mut() {
        Some(player) => {
            if player.remove_from_queue(index) {
                Ok("Song aus Queue entfernt".to_string())
            } else {
                Err(format!("Ungültiger Index: {}", index))
            }
        }
        None => Err("Player nicht initialisiert".to_string()),
    }
}

/// Returns the song queue.
#[tauri::command]
fn get_queue(state: tauri::State<AppState>) -> Vec<String> {
    let p = state.player.lock().unwrap();
    match p.as_ref() {
        Some(player) => player.get_queue(),
        None => vec![],
    }
}

/// Sets the playback volume (0.0 = silent, 1.0 = full).
#[tauri::command]
fn set_volume(state: tauri::State<AppState>, volume: f32) -> Result<String, String> {
    let p = state.player.lock().unwrap();
    match p.as_ref() {
        Some(player) => {
            player.set_volume(volume);
            Ok(format!("Lautstärke auf {:.0}% gesetzt", volume * 100.0))
        }
        None => Err("Player nicht initialisiert".to_string()),
    }
}

/// Wird regelmäßig vom Frontend aufgerufen.
/// Prüft ob der aktuelle Song beendet ist und startet ggf. den nächsten.
/// Gibt true zurück, wenn ein neuer Song gestartet wurde.
#[tauri::command]
fn tick(state: tauri::State<AppState>) -> Result<bool, String> {
    let mut p = state.player.lock().unwrap();
    match p.as_mut() {
        Some(player) if !player.is_paused => player
            .advance_queue()
            .map_err(|e| format!("Fehler beim Weiterschalten: {}", e)),
        _ => Ok(false),
    }
}
