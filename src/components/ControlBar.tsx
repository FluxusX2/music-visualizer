import {invoke} from "@tauri-apps/api/core";
import {useState} from "react";
import { Play, Pause, SkipForward, SkipBack } from "lucide-react";

function ControlBar() {
    // Audio starts playing automatically on launch, so we begin in "playing" state.
    const [isPaused, setIsPaused] = useState(false);

    function toggle_playback() {
        invoke<boolean>("toggle_playback")
            .then(newPausedState => setIsPaused(newPausedState))
            .catch(e => console.error("toggle_playback failed:", e));
    }

    return (
        <div className={"control-bar"}>
            <button>
                <SkipBack className={"icons"}/>
            </button>
            <button className={"play-button"} onClick={toggle_playback}>
                {isPaused ? <Play className={"icons"}/> : <Pause className={"icons"}/>}
            </button>
            <button>
                <SkipForward className={"icons"}/>
            </button>
        </div>
    );
}

export default ControlBar;

