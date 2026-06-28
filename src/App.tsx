
import "./App.css";
import {invoke} from "@tauri-apps/api/core";
import {useEffect, useState} from "react";

function App() {
  const [isPlaying, setIsPlaying] = useState(false);
  const [files, setFiles] = useState<string[]>([])

  useEffect(() => {
    void loadFiles();
  }, []);

  async function loadFiles() {
    try {
      const result = await invoke<string[]>("list_files", {
        folder: "C:\\Users\\maelb\\Desktop\\music",
      });
      setFiles(result);
    } catch (error) {
      console.error(error);
    }
  }

  async function callRust() {
    try {
      if (isPlaying) {
        await invoke<string>("stop_sound");
        setIsPlaying(false);
      } else {
        await invoke<string>("play_sound");
        setIsPlaying(true);
      }
    } catch (error) {
      console.error(error);
    }
  }



  return (
    <main className="container">

      <button
          onClick={callRust}
      >
        {isPlaying ? "Pause" : "Play"}
      </button>
      <ul>
        {files.map((file) => (
            <li key={file}>{file}</li>
        ))}
      </ul>
    </main>
  );
}

export default App;
