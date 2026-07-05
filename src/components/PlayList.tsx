import {invoke} from "@tauri-apps/api/core";
import {useEffect, useState} from "react";
import PlayListCard from "./PlayListCard.tsx";

export default function PlayList() {
    const [songs, setSongs] = useState<string[][]>([])
    const [error, setError] = useState<string | null>(null)

    const fetchSongs = async () => {
        try {
            // 'dirStr' ist der Parametername 'dir_str' in camelCase
            const targetPath = "../music";

            const result = await invoke<string[][]>('scan_dir', { dirStr: targetPath });
            setSongs(result);
            setError(null);
        } catch (err) {
            console.error(err);
            setError(err as string);
        }
    };

    useEffect(() => {
        fetchSongs().then();
    }, [])

    return (
        <div className={"playlist"}>

            <ul className={"song-list"}>
                {songs.map((song, index) => (
                    <li key={index}>
                        <PlayListCard song={song} />
                    </li>
                ))}
            </ul>
            {error && <p className="error">{error}</p>}

        </div>
    );

}