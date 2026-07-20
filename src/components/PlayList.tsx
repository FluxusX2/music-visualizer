import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import PlayListCard from "./PlayListCard.tsx";

// Define the shape that matches your Rust struct
export interface SongInfo {
    file_name: string;
    path: string;
    title: string | null;
    artist: string | null;
    cover_art: number[] | null; // Vec<u8> becomes an array of numbers in JS
}

export default function PlayList({ onSongSelect }: { onSongSelect?: (song: SongInfo) => void }) {
    // Update the state type
    const [songs, setSongs] = useState<SongInfo[]>([]);
    const [error, setError] = useState<string | null>(null);

    const fetchSongs = async () => {
        try {
            const targetPath = "../music";

            // Invoke now expects an array of SongInfo objects
            const result = await invoke<SongInfo[]>('scan_dir', { dirStr: targetPath });
            setSongs(result);
            setError(null);
        } catch (err) {
            console.error(err);
            setError(err as string);
        }
    };

    useEffect(() => {
        fetchSongs().then();
    }, []);

    return (
        <div className="playlist">
            <ul className="song-list">
                {songs.map((song) => (
                    // Using the path as a key is usually safer than index if songs can be reordered
                    <li key={song.path}>
                        <PlayListCard song={song} onSelect={onSongSelect} />
                    </li>
                ))}
            </ul>
            {error && <p className="error">{error}</p>}
        </div>
    );
}