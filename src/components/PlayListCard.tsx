import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { SongInfo } from "./PlayList.tsx";

export default function PlayListCard({ song }: { song: SongInfo }) {
    const [coverUrl, setCoverUrl] = useState<string | null>(null);

    useEffect(() => {
        // If we have cover art bytes, convert them to a usable Image URL
        if (song.cover_art && song.cover_art.length > 0) {
            const byteArray = new Uint8Array(song.cover_art);
            const blob = new Blob([byteArray]);
            const url = URL.createObjectURL(blob);
            setCoverUrl(url);
            return () => {
                URL.revokeObjectURL(url);
            };
        }
    }, [song.cover_art]);

    const loadSong = async () => {
        await invoke('load_song', { dir: song.path });
    };

    return (
        <div>
            <button
                className="playlist-card"
                onClick={loadSong}
            >
                {/* Display the Cover Art or a fallback */}
                <div className="cover-art" style={{ width: '50px', height: '50px', flexShrink: 0, backgroundColor: '#333' }}>
                    {coverUrl ? (
                        <img
                            src={coverUrl}
                            alt={`${song.title || song.file_name} cover`}
                            style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                        />
                    ) : (
                        <div style={{ display: 'flex', width: '100%', height: '100%', alignItems: 'center', justifyContent: 'center', color: '#888' }}>
                            🎵
                        </div>
                    )}
                </div>

                {/* Display the Metadata */}
                <div className="metadata" style={{ textAlign: 'left' }}>
                    <div style={{ fontWeight: 'bold' }}>
                        {song.title || song.file_name}
                    </div>
                    <div style={{ fontSize: '0.85em', opacity: 0.8 }}>
                        {song.artist || "Unknown Artist"}
                    </div>
                </div>
            </button>
        </div>
    );
}