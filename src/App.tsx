import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useCallback } from "react";
import PlayList from "./components/PlayList.tsx";
import Controls from "./components/Controls.tsx";

const MUSIC_FOLDER = "C:\\Users\\maelb\\Desktop\\music";

function App() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [files, setFiles] = useState<string[]>([]);
  const [queue, setQueue] = useState<string[]>([]);
  const [volume, setVolume] = useState(1.0);

  const refreshQueue = useCallback(async () => {
    try {
      const result = await invoke<string[]>("get_queue");
      setQueue(result);
    } catch (error) {
      console.error("Fehler beim Laden der Queue:", error);
    }
  }, []);

  useEffect(() => {
    void loadFiles();
  }, []);

  // Alle 500 ms: Song-Ende prüfen (tick) + Queue aktualisieren
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        await invoke("tick");
      } catch {
        // ignorieren
      }
      void refreshQueue();
    }, 500);
    return () => clearInterval(interval);
  }, [refreshQueue]);

  async function loadFiles() {
    try {
      const result = await invoke<string[]>("list_files", {
        folder: MUSIC_FOLDER,
      });
      setFiles(result);
    } catch (error) {
      console.error("Fehler beim Laden der Dateien:", error);
    }
  }

  async function togglePlayback() {
    try {
      if (isPlaying) {
        await invoke("stop_sound");
        setIsPlaying(false);
      } else {
        await invoke("play_sound");
        setIsPlaying(true);
      }
    } catch (error) {
      console.error(error);
    }
  }

  async function addToQueue(path: string) {
    try {
      await invoke("add_to_queue", { path });
      // Wenn kein Song lief, ist jetzt einer aktiv
      setIsPlaying(true);
      await refreshQueue();
    } catch (error) {
      console.error("Fehler beim Hinzufügen zur Queue:", error);
    }
  }

  async function removeFromQueue(index: number) {
    try {
      await invoke("remove_from_queue", { index });
      await refreshQueue();
    } catch (error) {
      console.error("Fehler beim Entfernen aus der Queue:", error);
    }
  }

  async function handleVolumeChange(vol: number) {
    setVolume(vol);
    try {
      await invoke("set_volume", { volume: vol });
    } catch (error) {
      console.error("Fehler beim Setzen der Lautstärke:", error);
    }
  }

  return (
    <main className="container">
      <Controls isPlaying={isPlaying} onToggle={togglePlayback} volume={volume} onVolumeChange={handleVolumeChange} />
      <PlayList
        files={files}
        queue={queue}
        onAddToQueue={addToQueue}
        onRemoveFromQueue={removeFromQueue}
      />
    </main>
  );
}

export default App;