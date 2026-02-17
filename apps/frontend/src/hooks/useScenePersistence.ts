import { useMutation, useQuery } from "@tanstack/react-query"
import { ConceptCluster } from "@/App"

export function useSaveScene() {
    return useMutation({
        mutationFn: async ({ sceneData, sceneId }: { sceneData: ConceptCluster[]; sceneId?: string }) => {
            const res = await fetch('http://localhost:8000/api/scenes', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    scene_data: sceneData,
                    scene_id: sceneId || undefined,
                }),
            });

            if (!res.ok) {
                const body = await res.json().catch(() => null);
                throw new Error(body?.detail || `Server error (${res.status})`);
            }

            const data = await res.json();
            return data.data.scene_id as string;
        },
    });
}

export function useLoadScene(sceneId: string | null) {
    return useQuery<ConceptCluster[]>({
        queryKey: ['scene', sceneId],
        queryFn: async () => {
            const res = await fetch(`http://localhost:8000/api/scenes/${sceneId}`);

            if (res.status === 404) {
                throw new Error('Scene not found');
            }

            if (!res.ok) {
                const body = await res.json().catch(() => null);
                throw new Error(body?.detail || `Server error (${res.status})`);
            }

            const data = await res.json();
            return data.data as ConceptCluster[];
        },
        enabled: !!sceneId,
        staleTime: Infinity,
        retry: false,
    });
}
