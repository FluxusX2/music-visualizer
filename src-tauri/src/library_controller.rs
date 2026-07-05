use std::fs;
use std::fs::create_dir;
use std::path::Path;

pub fn scan_dir(dir_str: String) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let mut playlist = Vec::new();

    let dir = Path::new(&dir_str);
    if !dir.is_dir() {
        create_dir(dir)?;
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_dir = entry.path();
        if file_dir.is_file() {
            let file_dir_str = file_dir.to_str().unwrap();
            if file_dir_str.ends_with(".flac") {
                let name = file_dir.file_stem().unwrap().to_str().unwrap().to_string();
                println!("{:?}, {:?}", name, file_dir_str);
                let file_vec = vec![name, file_dir_str.to_string()];
                playlist.push(file_vec);
            }
        }
    }

    Ok(playlist)
}