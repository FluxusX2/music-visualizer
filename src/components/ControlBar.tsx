import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {useEffect, useState} from "react";
import { Play, Pause, SkipForward, SkipBack } from "lucide-react";

function ControlBar() {
    const [isPaused, setIsPaused] = useState(true);

    useEffect(() => {
        let unlisten: (() => void) | undefined;

        listen<boolean>("playback-state", (event) => {
            setIsPaused(event.payload);
        }).then((fn) => {
            unlisten = fn;
        });

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, []);

    function toggle_playback() {
        invoke<boolean>("toggle_playback")
            .then(newPausedState => setIsPaused(newPausedState))
            .catch(e => console.error("toggle_playback failed:", e));
    }

    function skip_forward() {
        invoke("skip_forward")
            .catch(e => console.error("skip_forward failed:", e));
    }

    function skip_backward() {
        invoke("skip_backward")
            .catch(e => console.error("skip_backward failed:", e));
    }

    return (
        <div className={"control-bar"}>
            <button onClick={skip_backward}>
                <SkipBack className={"icons"}/>
            </button>
            <button className={"play-button"} onClick={toggle_playback}>
                {isPaused ? <Play className={"icons"}/> : <Pause className={"icons"}/>}
            </button>
            <button onClick={skip_forward}>
                <SkipForward className={"icons"}/>
            </button>
        </div>
    );
}

export default ControlBar;
