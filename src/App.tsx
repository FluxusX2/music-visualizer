import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import ControlBar from "./components/ControlBar";
import PlayList, { type SongInfo } from "./components/PlayList.tsx";
import Content from "./components/Content.tsx";
import "./styles/scrollbar.css";

function App() {
  const [songs, setSongs] = useState<SongInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [currentSong, setCurrentSong] = useState<SongInfo | null>(null);

  // Load the song library once on startup.
  useEffect(() => {
    const fetchSongs = async () => {
      try {
        const targetPath = "../music";
        const result = await invoke<SongInfo[]>("scan_dir", { dirStr: targetPath });
        setSongs(result);
        setError(null);
      } catch (err) {
        console.error(err);
        setError(err as string);
      }
    };
    fetchSongs().then();
  }, []);

  // The backend emits the path of the song that is actually loaded/playing
  // (e.g. after skip forward/backward or when the queue auto-advances).
  // Keep the displayed cover art in sync with that, instead of whatever was
  // last added to the queue.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("song-changed", (event) => {
      const path = event.payload;
      if (!path) {
        setCurrentSong(null);
        return;
      }
      setSongs((current) => {
        const match = current.find((s) => s.path === path);
        if (match) {
          setCurrentSong(match);
        }
        return current;
      });
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return (
    <main className="container">
        <ControlBar/>
        <PlayList songs={songs} error={error}/>
        <div className={"content"}>
            <Content song={currentSong}/>
        </div>
    </main>
  );
}

export default App;