use std::fs::{self, create_dir};
use std::path::Path;
use lofty::read_from_path;
use lofty::prelude::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SongInfo {
    pub file_name: String,
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub cover_art: Option<Vec<u8>>,
}

// A list of common audio extensions supported by lofty
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "m4a", "mp4", "aac", "wma", "aiff", "ape"
];

pub fn scan_dir(dir_str: &str) -> Result<Vec<SongInfo>, Box<dyn std::error::Error>> {
    let mut playlist = Vec::new();
    let dir = Path::new(dir_str);

    if !dir.is_dir() {
        create_dir(dir)?;
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            // Get the extension, convert to lowercase to handle ".MP3" vs ".mp3"
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            // Only process if it matches our list of audio extensions
            if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                let file_name = file_path.file_stem().unwrap().to_string_lossy().into_owned();
                let path_str = file_path.to_string_lossy().into_owned();

                let mut song = SongInfo {
                    file_name,
                    path: path_str,
                    title: None,
                    artist: None,
                    cover_art: None,
                };

                if let Ok(tagged_file) = read_from_path(&file_path) {
                    // Try to get the primary tag, fallback to the first available tag if none is marked as primary
                    let tag = tagged_file
                        .primary_tag()
                        .or_else(|| tagged_file.first_tag());

                    if let Some(t) = tag {
                        song.title = t.title().map(|s| s.into_owned());
                        song.artist = t.artist().map(|s| s.into_owned());

                        if let Some(picture) = t.pictures().first() {
                            song.cover_art = Some(picture.data().to_vec());
                        }
                    }
                }

                println!("Found audio file: {:?}", song.file_name);
                playlist.push(song);
            }
        }
    }

    Ok(playlist)
}