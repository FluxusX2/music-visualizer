import PlayListCard from "./PlayListCard.tsx";

// Define the shape that matches your Rust struct
export interface SongInfo {
    file_name: string;
    path: string;
    title: string | null;
    artist: string | null;
    cover_art: number[] | null; // Vec<u8> becomes an array of numbers in JS
}

export default function PlayList({ songs, error }: {
    songs: SongInfo[];
    error?: string | null;
}) {
    return (
        <div className="playlist custom-scroll">
            <ul className="song-list">
                {songs.map((song) => (
                    // Using the path as a key is usually safer than index if songs can be reordered
                    <li key={song.path}>
                        <PlayListCard song={song} />
                    </li>
                ))}
            </ul>
            {error && <p className="error">{error}</p>}
        </div>
    );
}