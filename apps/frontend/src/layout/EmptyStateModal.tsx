import { Upload, Link } from "lucide-react"
import { useRef, useState } from "react"
import { useFileUpload } from "@/hooks/useFileUpload"
import { useUrlUpload } from "@/hooks/useUrlUpload"

type EmptyStateModalProps = {
    onSimulationUpdate: (data: any) => void;
    setLoadingState: (loading: boolean) => void;
}

export function EmptyStateModal({ onSimulationUpdate, setLoadingState }: EmptyStateModalProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);
    const { uploadFile, isPending } = useFileUpload({ onSimulationUpdate, setLoadingState });
    const { uploadUrl, isPending: isUrlPending } = useUrlUpload({ onSimulationUpdate, setLoadingState });
    const [urlValue, setUrlValue] = useState("");

    const anyPending = isPending || isUrlPending;

    const handleUrlSubmit = () => {
        const trimmed = urlValue.trim();
        if (!trimmed) return;
        uploadUrl(trimmed);
        setUrlValue("");
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm">
            <div className="bg-terminal-bg/90 backdrop-blur-lg border border-terminal-border rounded-lg p-8 max-w-md w-full mx-4">
                {/* Terminal header bar */}
                <div className="flex items-center gap-2 mb-6 pb-3 border-b border-terminal-border/50">
                    <span className="inline-block w-2 h-2 bg-terminal-text animate-terminal-blink"></span>
                    <span className="text-terminal-text text-xs tracking-wider">OORT TERMINAL v1.0.0</span>
                </div>

                {/* Main content */}
                <div className="text-center space-y-4">
                    <h2 className="text-terminal-text text-lg">
                        Your universe is still empty.
                    </h2>
                    <p className="text-terminal-muted text-sm">
                        Start creating planets by uploading a text file or pasting an article URL.
                    </p>

                    {/* Upload button */}
                    <button
                        onClick={() => fileInputRef.current?.click()}
                        disabled={anyPending}
                        className="mt-6 inline-flex items-center gap-2 px-6 py-3 border border-terminal-border rounded-md text-terminal-text text-sm transition-colors hover:bg-zinc-800 hover:border-green-700/50 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {anyPending ? (
                            <>
                                <span className="inline-block w-2 h-2 bg-terminal-text animate-terminal-blink"></span>
                                Processing...
                            </>
                        ) : (
                            <>
                                <Upload size={16} />
                                Upload Text File
                            </>
                        )}
                    </button>
                    <input
                        ref={fileInputRef}
                        type="file"
                        accept=".txt,.md,.text"
                        className="hidden"
                        onChange={uploadFile}
                        disabled={anyPending}
                    />

                    {/* OR divider */}
                    <div className="flex items-center gap-3 my-2">
                        <div className="flex-1 border-t border-terminal-border/50"></div>
                        <span className="text-terminal-muted text-xs">OR</span>
                        <div className="flex-1 border-t border-terminal-border/50"></div>
                    </div>

                    {/* URL input */}
                    <div className="flex items-center gap-2 border border-terminal-border rounded-md px-3 py-2">
                        <Link size={14} className="text-terminal-muted shrink-0" />
                        <input
                            type="text"
                            value={urlValue}
                            onChange={e => setUrlValue(e.target.value)}
                            onKeyDown={e => e.key === 'Enter' && handleUrlSubmit()}
                            placeholder="https://..."
                            disabled={anyPending}
                            className="flex-1 bg-transparent border-none outline-none text-terminal-text placeholder-terminal-muted text-sm"
                        />
                        <button
                            onClick={handleUrlSubmit}
                            disabled={anyPending || !urlValue.trim()}
                            className="text-xs text-terminal-text border border-terminal-border px-2 py-1 rounded hover:bg-zinc-800 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            GO
                        </button>
                    </div>
                </div>
            </div>
        </div>
    )
}
