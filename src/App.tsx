import "./App.css";
import ControlBar from "./components/ControlBar";
import PlayList from "./components/PlayList.tsx";

function App() {



  return (
    <main className="container">
        <ControlBar/>
        <PlayList/>
        <div className={"content"}>

        </div>
    </main>
  );
}

export default App;