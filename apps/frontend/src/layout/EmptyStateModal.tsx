import { Upload } from "lucide-react"
import { useRef } from "react"
import { useFileUpload } from "@/hooks/useFileUpload"

type EmptyStateModalProps = {
    onSimulationUpdate: (data: any) => void;
    setLoadingState: (loading: boolean) => void;
}

export function EmptyStateModal({ onSimulationUpdate, setLoadingState }: EmptyStateModalProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);
    const { uploadFile, isPending } = useFileUpload({ onSimulationUpdate, setLoadingState });

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
                        Start creating planets by uploading a text file.
                    </p>

                    {/* Upload button */}
                    <button
                        onClick={() => fileInputRef.current?.click()}
                        disabled={isPending}
                        className="mt-6 inline-flex items-center gap-2 px-6 py-3 border border-terminal-border rounded-md text-terminal-text text-sm transition-colors hover:bg-zinc-800 hover:border-green-700/50 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {isPending ? (
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
                        disabled={isPending}
                    />
                </div>
            </div>
        </div>
    )
}
