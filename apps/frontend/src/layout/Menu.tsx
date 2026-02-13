import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import { cn } from "@/lib/utils"
import { Upload, Link } from "lucide-react"
import { ConceptCluster, Simulation } from "@/App"
import { useFileUpload } from "@/hooks/useFileUpload"
import { useUrlUpload } from "@/hooks/useUrlUpload"
import { useState } from "react"

export const Menu = ({ concepts, onSelect, active, onSimulationUpdate, setLoadingState }: {
    concepts: Simulation,
    onSelect: (concept: string) => void,
    active: string,
    onSimulationUpdate: (data: any) => void,
    setLoadingState: (loading: boolean) => void
}) => {
    const { uploadFile, isPending } = useFileUpload({ onSimulationUpdate, setLoadingState })
    const { uploadUrl, isPending: isUrlPending } = useUrlUpload({ onSimulationUpdate, setLoadingState })
    const [showUrlInput, setShowUrlInput] = useState(false)
    const [urlValue, setUrlValue] = useState("")

    const anyPending = isPending || isUrlPending

    const handleUrlSubmit = () => {
        const trimmed = urlValue.trim()
        if (!trimmed) return
        uploadUrl(trimmed)
        setUrlValue("")
        setShowUrlInput(false)
    }

    // Scale factor must match Scene.tsx
    const SCENE_SCALE = 2;

    return (
        <div className="[>*]:text-[32px] bg-terminal-bg bx-2 by-2 border-terminal-border shadow-lg overflow-hidden relative h-full mx-auto">
            <div className="relative h-full">
                <div className="absolute inset-0 pointer-events-none z-10 bg-[repeating-linear-gradient(0deg,rgba(0,0,0,0.15),rgba(0,0,0,0.15)_1px,transparent_1px,transparent_2px)]"></div>
                <div className="absolute inset-0 pointer-events-none z-20 bg-[radial-gradient(ellipse_at_center,transparent_0%,rgba(0,0,0,0.2)_90%,rgba(0,0,0,0.4)_100%)]"></div>

                <Command className="rounded-none border-none bg-transparent h-full flex flex-col">
                    <div className="border-y border-terminal-border px-3 py-1 bg-terminal-bg flex justify-between items-center">
                        <div className="border-none outline-none text-terminal-text bg-transparent placeholder-terminal-muted caret-terminal-text text-[16px] w-lg">
                            <CommandInput placeholder="Search concepts..." />
                        </div>
                        <div className="flex items-center gap-1">
                            <button
                                onClick={() => setShowUrlInput(v => !v)}
                                disabled={anyPending}
                                className={`flex items-center justify-center w-10 h-10 rounded-md transition-colors ${anyPending ? 'text-gray-500' : 'text-terminal-text hover:text-green-300 hover:bg-zinc-800'}`}
                            >
                                <Link size={20} />
                            </button>
                            <label
                                htmlFor="file-upload"
                                className={`cursor-pointer flex items-center justify-center w-10 h-10 rounded-md transition-colors ${anyPending ? 'text-gray-500' : 'text-terminal-text hover:text-green-300 hover:bg-zinc-800'}`}
                            >
                                <Upload size={20} />
                                <input
                                    id="file-upload"
                                    type="file"
                                    accept=".txt,.md,.text"
                                    className="hidden"
                                    onChange={uploadFile}
                                    disabled={anyPending}
                                />
                            </label>
                        </div>
                    </div>

                    {showUrlInput && (
                        <div className="border-b border-terminal-border px-3 py-2 bg-terminal-bg flex items-center gap-2">
                            <Link size={14} className="text-terminal-muted shrink-0" />
                            <input
                                type="text"
                                value={urlValue}
                                onChange={e => setUrlValue(e.target.value)}
                                onKeyDown={e => e.key === 'Enter' && handleUrlSubmit()}
                                placeholder="https://..."
                                disabled={anyPending}
                                className="flex-1 bg-transparent border-none outline-none text-terminal-text placeholder-terminal-muted text-sm"
                                autoFocus
                            />
                            <button
                                onClick={handleUrlSubmit}
                                disabled={anyPending || !urlValue.trim()}
                                className="text-xs text-terminal-text border border-terminal-border px-2 py-1 rounded hover:bg-zinc-800 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                GO
                            </button>
                        </div>
                    )}

                    <CommandList className="flex-1 overflow-y-auto scrollbar scrollbar-w-3 scrollbar-thumb-terminal-border scrollbar-track-zinc-800">
                        <CommandEmpty className="px-4 py-8 text-center text-terminal-muted">No results found.</CommandEmpty>
                        <CommandGroup className="bg-terminal-bg text-terminal-text">
                            {concepts.map((concept: ConceptCluster) => {
                                const safeEmbedding = concept.reduced_embedding.map(p => {
                                    let val = typeof p === 'string' ? parseFloat(p) : Number(p);
                                    return isNaN(val) ? 0 : val * SCENE_SCALE;
                                });
                                const conceptKey = safeEmbedding.map(String).join("-");

                                return concept.concepts.map((individualConcept) => (
                                    <CommandItem
                                        onSelect={() => onSelect(individualConcept)}
                                        key={individualConcept}
                                        className={cn(
                                            "px-4 py-3 cursor-pointer transition-colors text-[32px]",
                                            active === conceptKey
                                                ? "bg-zinc-800 "
                                                : "text-terminal-text hover:bg-zinc-800 hover:text-green-300",
                                        )}
                                    >
                                        <span>{individualConcept}</span>
                                    </CommandItem>
                                ))
                            })}
                        </CommandGroup>
                    </CommandList>

                    <div className="bg-zinc-800 border-t border-terminal-border px-4 py-2 text-xs text-terminal-text flex justify-between items-center">
                        <div className="flex items-center">
                            <span className="inline-block w-2 h-2 bg-terminal-text mr-2 animate-terminal-blink"></span>
                            OORT TERMINAL v1.0.0
                        </div>
                        <div className="flex gap-4">
                            <span>ARROWS: NAVIGATE</span>
                            <span>ENTER: SELECT</span>
                        </div>
                    </div>
                </Command>
            </div>
        </div>
    )
}
