import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState, useRef } from "react";
import { Play, Pause, SkipForward, SkipBack } from "lucide-react";
import { Slider } from "@mui/material";

interface PlaybackProgress {
    position: number;
    duration: number;
}

function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds < 0) {
        seconds = 0;
    }
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
}

function ControlBar() {
    const [isPaused, setIsPaused] = useState(true);
    const [position, setPosition] = useState(0);
    const [duration, setDuration] = useState(0);
    const isSeeking = useRef(false);

    useEffect(() => {
        let unlistenState: (() => void) | undefined;
        let unlistenProgress: (() => void) | undefined;

        listen<boolean>("playback-state", (event) => {
            setIsPaused(event.payload);
        }).then((fn) => {
            unlistenState = fn;
        });

        listen<PlaybackProgress>("playback-progress", (event) => {
            // Read directly from the ref. If the user is dragging, ignore backend updates.
            if (isSeeking.current) {
                return;
            }
            setPosition(event.payload.position);
            setDuration(event.payload.duration);
        }).then((fn) => {
            unlistenProgress = fn;
        });

        return () => {
            if (unlistenState) unlistenState();
            if (unlistenProgress) unlistenProgress();
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

    function handleVolumeChange(_event: Event, newValue: number | number[]) {
        invoke("set_volume", { volume: ((newValue as number) / 100) })
            .catch(e => console.error("set_volume failed:", e));
    }

    function handleProgressChange(_event: Event, newValue: number | number[]) {
        // Instantly block backend updates
        isSeeking.current = true;
        setPosition(newValue as number);
    }

    function handleProgressCommitted(_event: React.SyntheticEvent | Event, newValue: number | number[]) {
        invoke("seek", { positionSecs: newValue as number })
            .catch(e => console.error("seek failed:", e))
            .finally(() => {
                isSeeking.current = false;
            });
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
            <Slider valueLabelDisplay={"auto"}
                    className={"progress-bar"}
                    value={position}
                    min={0}
                    max={duration || 0}
                    onChange={handleProgressChange}
                    onChangeCommitted={handleProgressCommitted}
                    valueLabelFormat={formatTime}
            />
            <Slider orientation={"horizontal"}
                    valueLabelDisplay={"auto"}
                    onChange={handleVolumeChange}
                    defaultValue={50}
                    className={"volume-slider"}/>
        </div>
    );
}

export default ControlBar;