interface FileListProps {
    files: string[];
    queue: string[];
    onAddToQueue: (path: string) => void;
    onRemoveFromQueue: (index: number) => void;
}

function FileList({ files, queue, onAddToQueue, onRemoveFromQueue }: FileListProps) {
    const getFileName = (path: string) => path.split("\\").pop() ?? path;

    return (
        <div className="playlist-container">
            <section>
                <h3>Verfügbare Dateien</h3>
                {files.length === 0 ? (
                    <p className="empty-hint">Keine Dateien gefunden.</p>
                ) : (
                    <ul className="file-list">
                        {files.map((file) => (
                            <li key={file} className="file-item">
                                <span className="file-name">{getFileName(file)}</span>
                                <button
                                    className="btn-add"
                                    onClick={() => onAddToQueue(file)}
                                    title="Zur Queue hinzufügen"
                                >
                                    ＋
                                </button>
                            </li>
                        ))}
                    </ul>
                )}
            </section>

            <section>
                <h3>Wiedergabe-Queue</h3>
                {queue.length === 0 ? (
                    <p className="empty-hint">Queue ist leer.</p>
                ) : (
                    <ol className="queue-list">
                        {queue.map((file, i) => (
                            <li key={`${file}-${i}`} className="queue-item">
                                <span className="file-name">{getFileName(file)}</span>
                                <button
                                    className="btn-remove"
                                    onClick={() => onRemoveFromQueue(i)}
                                    title="Aus Queue entfernen"
                                >
                                    ✕
                                </button>
                            </li>
                        ))}
                    </ol>
                )}
            </section>
        </div>
    );
}

export default FileList;