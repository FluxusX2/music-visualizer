import { useState } from "react";
import "./App.css";
import ControlBar from "./components/ControlBar";
import PlayList, { type SongInfo } from "./components/PlayList.tsx";
import Content from "./components/Content.tsx";

function App() {
  const [currentSong, setCurrentSong] = useState<SongInfo | null>(null);

  return (
    <main className="container">
        <ControlBar/>
        <PlayList onSongSelect={setCurrentSong}/>
        <div className={"content"}>
            <Content song={currentSong}/>
        </div>
    </main>
  );
}

export default App;