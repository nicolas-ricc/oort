import Render from './cloud/Render'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'
import { FloatingPlanetPanel } from './layout/FloatingPlanetPanel'
import { EmptyStateModal } from './layout/EmptyStateModal'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useNavigation } from './hooks/useNavigation'
import { useSaveScene, useLoadScene } from './hooks/useScenePersistence'
import { ColorClusterInfo } from './cloud/Scene'

export type ConceptCluster = {
  concepts: string[];
  reduced_embedding: number[];
  cluster?: number;
  group_id?: number;
}

export type Simulation = ConceptCluster[]

// Stable key for a ConceptCluster, independent of position
function getNodeKey(node: ConceptCluster): string {
  return node.concepts.slice().sort().join("|");
}

function getSceneIdFromUrl(): string | null {
  return new URLSearchParams(window.location.search).get('scene');
}

function AppInner() {
  const [simulationData, setSimulationData] = useState<Simulation>([])
  const [active, setActive] = useState<string>("")
  const [isLoading, setIsLoading] = useState(false)
  const [isAnimating, setIsAnimating] = useState(false)
  const [colorClusterInfo, setColorClusterInfo] = useState<ColorClusterInfo | null>(null)
  const [currentSceneId, setCurrentSceneId] = useState<string | null>(getSceneIdFromUrl)

  const screenPositionRef = useRef<{ x: number; y: number } | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);

  const isEmpty = simulationData.length === 0;

  // Scene persistence
  const { mutate: saveSceneMutate, isPending: isSaving } = useSaveScene();
  const { data: loadedScene, isLoading: isSceneLoading, error: sceneError } = useLoadScene(currentSceneId);

  // Populate simulationData when a scene is loaded from URL
  useEffect(() => {
    if (loadedScene && loadedScene.length > 0) {
      setSimulationData(loadedScene);
      if (loadedScene[0]?.concepts?.length) {
        setActive(getNodeKey(loadedScene[0]));
      }
    }
  }, [loadedScene]);

  const handleSaveScene = useCallback(() => {
    if (simulationData.length === 0) return;

    saveSceneMutate(
      { sceneData: simulationData, sceneId: currentSceneId || undefined },
      {
        onSuccess: (sceneId) => {
          setCurrentSceneId(sceneId);
          window.history.replaceState({}, '', `?scene=${sceneId}`);
          navigator.clipboard.writeText(window.location.href).catch(() => {});
        },
        onError: (error) => {
          console.error('Error saving scene:', error);
          alert('Error saving scene. Please try again.');
        },
      }
    );
  }, [simulationData, currentSceneId, saveSceneMutate]);

  // Navigation hook
  const navigation = useNavigation(simulationData, active, setActive);

  // Get the currently selected cluster
  const selectedCluster = useMemo(() => {
    return simulationData.find(node => getNodeKey(node) === active) || null;
  }, [simulationData, active]);

  const handleColorClusterInfo = useCallback((info: ColorClusterInfo | null) => {
    setColorClusterInfo(info);
  }, []);

  const handleSimulationUpdate = useCallback((newData: ConceptCluster[]) => {
    if (!newData || !Array.isArray(newData) || newData.length === 0) return;
    setSimulationData(prev => {
      const existingKeys = new Set(prev.map(node => getNodeKey(node)));

      const uniqueNewData = newData.filter(node => {
        const key = getNodeKey(node);
        return key && !existingKeys.has(key);
      });

      return [...prev, ...uniqueNewData];
    });
    if (newData[0]?.concepts?.length) {
      setActive(getNodeKey(newData[0]));
    }
  }, [])

  const setLoadingState = useCallback((loading: boolean) => {
    setIsLoading(loading)
  }, [])

  const handleAnimatingChange = useCallback((animating: boolean) => {
    setIsAnimating(animating);
  }, []);

  // Scene loading state
  if (currentSceneId && isSceneLoading) {
    return (
      <div className="fixed inset-0 bg-black flex items-center justify-center">
        <div className="text-terminal-text text-sm flex items-center gap-2">
          <span className="inline-block w-2 h-2 bg-terminal-text animate-terminal-blink"></span>
          Loading scene...
        </div>
      </div>
    );
  }

  // Scene error state
  if (currentSceneId && sceneError) {
    return (
      <div className="fixed inset-0 bg-black flex items-center justify-center">
        <div className="text-center">
          <p className="text-terminal-text text-lg mb-4">Scene Not Found</p>
          <p className="text-terminal-muted text-sm mb-6">This scene may have been deleted or the link is invalid.</p>
          <a
            href="/"
            className="text-terminal-text border border-terminal-border px-4 py-2 rounded hover:bg-zinc-800 text-sm"
          >
            Start Fresh
          </a>
        </div>
      </div>
    );
  }

  return (
    <>
      <Layout isEmpty={isEmpty} canvasRef={canvasRef}>
        <Render
          simulation={simulationData}
          activeNode={active}
          setActive={setActive}
          onNavigateNext={navigation.navigateToNext}
          onNavigatePrevious={navigation.navigateToPrevious}
          onToggleTour={navigation.toggleTour}
          onResetToOverview={navigation.resetToOverview}
          onNavigateToIndex={navigation.navigateToIndex}
          screenPositionRef={screenPositionRef}
          onAnimatingChange={handleAnimatingChange}
          onColorClusterInfo={handleColorClusterInfo}
          isLoading={isLoading}
        />
        <Menu
          concepts={simulationData}
          onSelect={(concept) => {
            const foundNode = simulationData.find(s => s.concepts.includes(concept)) || simulationData[0];
            if (foundNode) {
              setActive(getNodeKey(foundNode));
            }
          }}
          onSimulationUpdate={handleSimulationUpdate}
          active={active}
          setLoadingState={setLoadingState}
          onSaveScene={handleSaveScene}
          isSaving={isSaving}
          currentSceneId={currentSceneId}
        />
      </Layout>

      {/* Floating panel - outside Layout, positioned fixed */}
      {!isEmpty && (
        <FloatingPlanetPanel
          selectedCluster={selectedCluster}
          clusterIndex={colorClusterInfo?.clusterIndex ?? 0}
          nearbyConcepts={colorClusterInfo?.nearbyConcepts ?? []}
          screenPositionRef={screenPositionRef}
          isAnimating={isAnimating}
          onClose={() => setActive("")}
        />
      )}

      {/* Empty state modal */}
      {isEmpty && !currentSceneId && !isSceneLoading && (
        <EmptyStateModal
          onSimulationUpdate={handleSimulationUpdate}
          setLoadingState={setLoadingState}
        />
      )}
    </>
  )
}

function App() {
  const client = useMemo(() => new QueryClient(), [])

  return (
    <QueryClientProvider client={client}>
      <AppInner />
    </QueryClientProvider>
  )
}

export default App
