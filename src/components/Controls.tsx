interface PlayerControlsProps {
    isPlaying: boolean;
    onToggle: () => void;
    volume: number;
    onVolumeChange: (vol: number) => void;
}

function PlayerControls({ isPlaying, onToggle, volume, onVolumeChange }: PlayerControlsProps) {
    return (
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <button onClick={onToggle}>
                {isPlaying ? "Pause" : "Play"}
            </button>
            <label style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                🔈
                <input
                    type="range"
                    min={0}
                    max={100}
                    value={Math.round(volume * 100)}
                    onChange={(e) => onVolumeChange(Number(e.target.value) / 100)}
                />
                🔊
                <span style={{ minWidth: "3ch" }}>{Math.round(volume * 100)}%</span>
            </label>
        </div>
    );
}

export default PlayerControls;