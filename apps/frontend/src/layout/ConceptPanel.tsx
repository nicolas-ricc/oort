import { ChevronLeft, ChevronRight, Play, Pause, ExternalLink, Circle } from "lucide-react"
import { useQuery } from "@tanstack/react-query"
import { ConceptCluster } from "@/App"
import { getAtmosphereColor } from "@/cloud/shaders/AtmosphereShader"

type TextReference = {
    text_id: string;
    user_id: string;
    filename: string;
    url: string;
    concepts: string[];
    upload_timestamp: string;
    file_size?: number;
}

type ConceptPanelProps = {
    selectedCluster: ConceptCluster | null;
    currentIndex: number;
    totalCount: number;
    tourMode: boolean;
    clusterIndex: number;
    onNavigateNext: () => void;
    onNavigatePrevious: () => void;
    onToggleTour: () => void;
    onNavigateToIndex: (index: number) => void;
}

export function ConceptPanel({
    selectedCluster,
    currentIndex,
    totalCount,
    tourMode,
    clusterIndex,
    onNavigateNext,
    onNavigatePrevious,
    onToggleTour,
    onNavigateToIndex
}: ConceptPanelProps) {
    const defaultUserId = "550e8400-e29b-41d4-a716-446655440000";
    const selectedConcept = selectedCluster?.concepts[0] || null;

    // Fetch text references for the first concept in the cluster
    const { data: textReferences } = useQuery({
        queryKey: ['textReferences', selectedConcept],
        queryFn: async () => {
            if (!selectedConcept) return [];
            const response = await fetch(
                `http://localhost:8000/api/texts-by-concept?concept=${encodeURIComponent(selectedConcept)}&user_id=${defaultUserId}`
            );
            if (!response.ok) throw new Error('Failed to fetch text references');
            const result = await response.json();
            return result.data as TextReference[];
        },
        enabled: !!selectedConcept,
    });

    const atmosphereColor = getAtmosphereColor(clusterIndex);

    return (
        <div className="h-full bg-terminal-bg border-l-2 border-terminal-border flex flex-col">
            {/* Header */}
            <div className="border-b border-terminal-border px-4 py-3 bg-zinc-900">
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                        <Circle
                            size={12}
                            fill={atmosphereColor}
                            color={atmosphereColor}
                        />
                        <span className="text-terminal-text text-sm font-medium">
                            Planet {currentIndex + 1}
                        </span>
                    </div>
                    <span className="text-terminal-muted text-xs">
                        Cluster {clusterIndex + 1}
                    </span>
                </div>
            </div>

            {/* Navigation Controls */}
            <div className="border-b border-terminal-border px-4 py-3 bg-zinc-900/50">
                <div className="flex items-center justify-between gap-2">
                    <button
                        onClick={onNavigatePrevious}
                        className="flex items-center justify-center w-8 h-8 rounded bg-zinc-800 hover:bg-zinc-700 text-terminal-text transition-colors"
                        title="Previous (P or Left Arrow)"
                    >
                        <ChevronLeft size={18} />
                    </button>

                    <div className="flex-1 text-center">
                        <span className="text-terminal-text text-sm">
                            {currentIndex + 1} / {totalCount}
                        </span>
                    </div>

                    <button
                        onClick={onNavigateNext}
                        className="flex items-center justify-center w-8 h-8 rounded bg-zinc-800 hover:bg-zinc-700 text-terminal-text transition-colors"
                        title="Next (N or Right Arrow)"
                    >
                        <ChevronRight size={18} />
                    </button>
                </div>

                {/* Tour Mode Toggle */}
                <button
                    onClick={onToggleTour}
                    className={`mt-3 w-full flex items-center justify-center gap-2 py-2 rounded transition-colors ${
                        tourMode
                            ? 'bg-green-700 hover:bg-green-600 text-white'
                            : 'bg-zinc-800 hover:bg-zinc-700 text-terminal-text'
                    }`}
                    title="Toggle Tour Mode (T)"
                >
                    {tourMode ? <Pause size={14} /> : <Play size={14} />}
                    <span className="text-sm">{tourMode ? 'Stop Tour' : 'Start Tour'}</span>
                </button>
            </div>

            {/* Concepts List */}
            <div className="flex-1 overflow-y-auto">
                {selectedCluster ? (
                    <div className="p-4">
                        <div className="flex items-center justify-between mb-3">
                            <h3 className="text-green-300 text-sm font-medium">Concepts</h3>
                            <span className="text-terminal-muted text-xs bg-zinc-800 px-2 py-0.5 rounded">
                                {selectedCluster.concepts.length}
                            </span>
                        </div>
                        <ul className="space-y-2">
                            {selectedCluster.concepts.map((concept, idx) => (
                                <li
                                    key={concept}
                                    className="flex items-start gap-2 text-terminal-text text-sm py-2 px-3 bg-zinc-800/50 rounded border border-zinc-700/50 hover:border-green-700/50 transition-colors"
                                >
                                    <span className="text-terminal-muted text-xs mt-0.5">{idx + 1}.</span>
                                    <span>{concept}</span>
                                </li>
                            ))}
                        </ul>
                    </div>
                ) : (
                    <div className="p-4 text-terminal-muted text-sm text-center">
                        Select a planet to view concepts
                    </div>
                )}
            </div>

            {/* Text References */}
            {selectedConcept && textReferences && textReferences.length > 0 && (
                <div className="border-t border-terminal-border">
                    <div className="px-4 py-2 bg-zinc-900 border-b border-terminal-border">
                        <span className="text-green-300 text-xs">Source Texts</span>
                    </div>
                    <div className="max-h-32 overflow-y-auto">
                        {textReferences.map((ref) => (
                            <div
                                key={ref.text_id}
                                className="px-4 py-2 text-xs text-terminal-text hover:bg-zinc-800 transition-colors border-b border-zinc-800/50"
                            >
                                <div className="flex items-center justify-between">
                                    <span className="truncate">{ref.filename}</span>
                                    <a
                                        href={ref.url}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        className="text-green-300 hover:text-green-400 ml-2 flex-shrink-0"
                                    >
                                        <ExternalLink size={12} />
                                    </a>
                                </div>
                                <div className="text-terminal-muted mt-1">
                                    {new Date(ref.upload_timestamp).toLocaleDateString()}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Quick Jump */}
            <div className="border-t border-terminal-border px-4 py-3 bg-zinc-900">
                <div className="text-terminal-muted text-xs mb-2">Quick Jump (1-9)</div>
                <div className="flex flex-wrap gap-1">
                    {Array.from({ length: Math.min(9, totalCount) }, (_, i) => (
                        <button
                            key={i}
                            onClick={() => onNavigateToIndex(i)}
                            className={`w-6 h-6 text-xs rounded transition-colors ${
                                currentIndex === i
                                    ? 'bg-green-700 text-white'
                                    : 'bg-zinc-800 text-terminal-text hover:bg-zinc-700'
                            }`}
                        >
                            {i + 1}
                        </button>
                    ))}
                    {totalCount > 9 && (
                        <span className="text-terminal-muted text-xs self-center ml-1">+{totalCount - 9}</span>
                    )}
                </div>
            </div>

            {/* Footer */}
            <div className="border-t border-terminal-border px-4 py-2 bg-zinc-800 text-terminal-muted text-xs">
                <div className="flex justify-between">
                    <span>ESC: Overview</span>
                    <span>T: Tour</span>
                </div>
            </div>
        </div>
    )
}
