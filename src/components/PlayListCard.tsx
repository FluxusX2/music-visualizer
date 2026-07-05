import {invoke} from "@tauri-apps/api/core";

export default function PlayListCard({ song }: { song: string[] }) {

    const loadSong = async (dir: String)=> {
        await invoke('load_song', { dir: dir });
    }

    return (
        <div>
            <button
                className={'playlist-card'}
                onClick={() => loadSong(song[1])}>
                <div>{song[0]}</div>
            </button>
        </div>
    )
}