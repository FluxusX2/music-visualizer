
import "./App.css";
import {invoke} from "@tauri-apps/api/core";

function App() {


  async function callRust() {
    await invoke<string>("play_sound");
  }

  return (
    <main className="container">

      <button
          onClick={callRust}
      />

    </main>
  );
}

export default App;
