import { MutableRefObject, useCallback, useEffect, useMemo, useRef, useState } from "react"
import { X, ExternalLink, Circle } from "lucide-react"
import { useQuery } from "@tanstack/react-query"
import { ConceptCluster } from "@/App"
import { getAtmosphereColor } from "@/cloud/shaders/AtmosphereShader"

type TextReference = {
    text_id: string;
    user_id: string;
    filename: string;
    url: string;
    source_url: string;
    concepts: string[];
    upload_timestamp: string;
    file_size?: number;
}

type FloatingPlanetPanelProps = {
    selectedCluster: ConceptCluster | null;
    clusterIndex: number;
    nearbyConcepts: string[][];
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
    nearbyConcepts,
    screenPositionRef,
    isAnimating,
    onClose
}: FloatingPlanetPanelProps) {
    const panelRef = useRef<HTMLDivElement>(null);
    const [position, setPosition] = useState<{ x: number; y: number } | null>(null);
    const visible = !isAnimating && selectedCluster !== null;

    const selectedConcept = selectedCluster?.concepts[0] || null;
    const subConcepts = selectedCluster?.concepts.slice(1) ?? [];

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

    // Deduplicate nearby concepts against this planet's concepts
    const filteredNearbyConcepts = useMemo(() => {
        const ownConcepts = new Set(selectedCluster?.concepts ?? []);
        const seen = new Set<string>();
        const result: string[] = [];
        for (const group of nearbyConcepts) {
            for (const c of group) {
                if (!ownConcepts.has(c) && !seen.has(c)) {
                    seen.add(c);
                    result.push(c);
                }
            }
        }
        return result;
    }, [nearbyConcepts, selectedCluster]);

    const sourceRefs = useMemo(() => {
        return textReferences ?? [];
    }, [textReferences]);

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
            <div className="bg-zinc-900/85 backdrop-blur-md border border-terminal-border/50 shadow-xl shadow-black/50 overflow-hidden flex flex-col" style={{ maxHeight: PANEL_MAX_HEIGHT }}>
                {/* Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b border-terminal-border/50">
                    <div className="flex items-center gap-2 min-w-0">
                        <Circle size={10} fill={atmosphereColor} color={atmosphereColor} className="flex-shrink-0" />
                        <span className="text-terminal-text text-sm font-medium truncate uppercase">
                            {selectedConcept ?? "Planet"}
                        </span>
                    </div>
                    <button
                        onClick={onClose}
                        className="text-terminal-muted hover:text-terminal-text transition-colors flex-shrink-0"
                    >
                        <X size={14} />
                    </button>
                </div>

                {/* Scrollable content */}
                <div className="overflow-y-auto flex-1 scrollbar scrollbar-w-1.5 scrollbar-thumb-terminal-border scrollbar-track-transparent">
                    {/* Sub-concepts (skip first/primary) */}
                    {subConcepts.length > 0 && (
                        <div className="px-4 py-3 space-y-1">
                            {subConcepts.map((concept) => (
                                <div
                                    key={concept}
                                    className="text-terminal-text text-xs font-mono"
                                >
                                    <span className="text-terminal-muted">{'> '}</span>{concept}
                                </div>
                            ))}
                        </div>
                    )}

                    {/* Nearby concepts from same-color planets */}
                    {filteredNearbyConcepts.length > 0 && (
                        <div className="border-t border-terminal-border/50">
                            <div className="px-4 pt-3 pb-1 flex items-center gap-2">
                                <span className="text-terminal-muted text-[10px] font-medium uppercase tracking-wider">NEARBY</span>
                                <div className="flex-1 border-t border-terminal-border/30" />
                            </div>
                            <div className="px-4 pb-3 space-y-1">
                                {filteredNearbyConcepts.map((concept) => (
                                    <div
                                        key={concept}
                                        className="text-terminal-muted text-xs font-mono"
                                    >
                                        <span className="text-terminal-border">{'> '}</span>{concept}
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Source references */}
                    {sourceRefs.length > 0 && (
                        <div className="border-t border-terminal-border/50">
                            <div className="px-4 pt-3 pb-1 flex items-center gap-2">
                                <span className="text-terminal-muted text-[10px] font-medium uppercase tracking-wider">SOURCES</span>
                                <div className="flex-1 border-t border-terminal-border/30" />
                            </div>
                            <div className="px-4 pb-3 space-y-1">
                                {sourceRefs.map((ref) =>
                                    ref.source_url && ref.source_url.length > 0 ? (
                                        <a
                                            key={ref.text_id}
                                            href={ref.source_url}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            className="flex items-center gap-2 text-xs text-terminal-text hover:text-green-300 transition-colors"
                                        >
                                            <span className="truncate">{ref.filename}</span>
                                            <ExternalLink size={11} className="flex-shrink-0 text-terminal-muted" />
                                        </a>
                                    ) : (
                                        <div
                                            key={ref.text_id}
                                            className="text-xs text-terminal-muted font-mono truncate"
                                        >
                                            {ref.filename}
                                        </div>
                                    )
                                )}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
