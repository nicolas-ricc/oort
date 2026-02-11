import { MutableRefObject, useCallback, useEffect, useRef, useState } from "react"
import { X, ExternalLink, Circle } from "lucide-react"
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

type FloatingPlanetPanelProps = {
    selectedCluster: ConceptCluster | null;
    clusterIndex: number;
    screenPositionRef: MutableRefObject<{ x: number; y: number } | null>;
    isAnimating: boolean;
    onClose: () => void;
}

const defaultUserId = "550e8400-e29b-41d4-a716-446655440000";
const PANEL_WIDTH = 320;
const PANEL_MAX_HEIGHT = 400;
const OFFSET_X = 30;

export function FloatingPlanetPanel({
    selectedCluster,
    clusterIndex,
    screenPositionRef,
    isAnimating,
    onClose
}: FloatingPlanetPanelProps) {
    const panelRef = useRef<HTMLDivElement>(null);
    const [position, setPosition] = useState<{ x: number; y: number } | null>(null);
    const visible = !isAnimating && selectedCluster !== null;

    const selectedConcept = selectedCluster?.concepts[0] || null;

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

    const updatePosition = useCallback(() => {
        const screenPos = screenPositionRef.current;
        if (!screenPos) {
            setPosition(null);
            return;
        }

        const vw = window.innerWidth;
        const vh = window.innerHeight;
        const panelHeight = panelRef.current?.offsetHeight ?? PANEL_MAX_HEIGHT;

        let x = screenPos.x + OFFSET_X;
        let y = screenPos.y - panelHeight / 2;

        // Flip to left side if overflowing right
        if (x + PANEL_WIDTH > vw - 16) {
            x = screenPos.x - OFFSET_X - PANEL_WIDTH;
        }

        // Clamp vertical
        y = Math.max(16, Math.min(y, vh - panelHeight - 16));

        setPosition({ x, y });
    }, [screenPositionRef]);

    useEffect(() => {
        if (!visible) return;
        let rafId: number;
        const loop = () => {
            updatePosition();
            rafId = requestAnimationFrame(loop);
        };
        rafId = requestAnimationFrame(loop);
        return () => cancelAnimationFrame(rafId);
    }, [visible, updatePosition]);

    if (!visible || !position) return null;

    return (
        <div
            ref={panelRef}
            className="fixed z-40 transition-opacity duration-200"
            style={{
                left: position.x,
                top: position.y,
                width: PANEL_WIDTH,
                maxHeight: PANEL_MAX_HEIGHT,
                opacity: visible ? 1 : 0,
            }}
        >
            <div className="bg-zinc-900/85 backdrop-blur-md border border-terminal-border/50 rounded-lg shadow-xl shadow-black/50 overflow-hidden flex flex-col" style={{ maxHeight: PANEL_MAX_HEIGHT }}>
                {/* Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b border-terminal-border/50">
                    <div className="flex items-center gap-2">
                        <Circle size={10} fill={atmosphereColor} color={atmosphereColor} />
                        <span className="text-terminal-text text-sm font-medium truncate">
                            {selectedCluster?.concepts[0] ?? "Planet"}
                        </span>
                    </div>
                    <button
                        onClick={onClose}
                        className="text-terminal-muted hover:text-terminal-text transition-colors"
                    >
                        <X size={14} />
                    </button>
                </div>

                {/* Scrollable content */}
                <div className="overflow-y-auto flex-1 scrollbar scrollbar-w-1.5 scrollbar-thumb-terminal-border scrollbar-track-transparent">
                    {/* Concepts */}
                    {selectedCluster && (
                        <div className="px-4 py-3">
                            <h3 className="text-green-300 text-xs font-medium mb-2">
                                Concepts ({selectedCluster.concepts.length})
                            </h3>
                            <ul className="space-y-1.5">
                                {selectedCluster.concepts.map((concept, idx) => (
                                    <li
                                        key={concept}
                                        className="flex items-start gap-2 text-terminal-text text-xs py-1.5 px-2.5 bg-zinc-800/50 rounded border border-zinc-700/50"
                                    >
                                        <span className="text-terminal-muted text-[10px] mt-0.5">{idx + 1}.</span>
                                        <span>{concept}</span>
                                    </li>
                                ))}
                            </ul>
                        </div>
                    )}

                    {/* Source Texts */}
                    {selectedConcept && textReferences && textReferences.length > 0 && (
                        <div className="border-t border-terminal-border/50">
                            <div className="px-4 py-2 bg-zinc-800/50">
                                <span className="text-green-300 text-xs">Source Texts</span>
                            </div>
                            {textReferences.map((ref) => (
                                <div
                                    key={ref.text_id}
                                    className="px-4 py-2 text-xs text-terminal-text hover:bg-zinc-800/50 transition-colors border-b border-zinc-800/30"
                                >
                                    <div className="flex items-center justify-between">
                                        <span className="truncate">{ref.filename}</span>
                                        <a
                                            href={ref.url}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="text-green-300 hover:text-green-400 ml-2 flex-shrink-0"
                                        >
                                            <ExternalLink size={11} />
                                        </a>
                                    </div>
                                    <div className="text-terminal-muted mt-0.5 text-[10px]">
                                        {new Date(ref.upload_timestamp).toLocaleDateString()}
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
