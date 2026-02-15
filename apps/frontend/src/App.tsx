import Render from './cloud/Render'
import { useCallback, useMemo, useRef, useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'
import { FloatingPlanetPanel } from './layout/FloatingPlanetPanel'
import { EmptyStateModal } from './layout/EmptyStateModal'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useNavigation } from './hooks/useNavigation'

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

function App() {
  const [simulationData, setSimulationData] = useState<Simulation>([])
  const [active, setActive] = useState<string>("")
  const [isLoading, setIsLoading] = useState(false)
  const [isAnimating, setIsAnimating] = useState(false)

  const screenPositionRef = useRef<{ x: number; y: number } | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);

  const isEmpty = simulationData.length === 0;

  // Navigation hook
  const navigation = useNavigation(simulationData, active, setActive);

  // Get the currently selected cluster
  const selectedCluster = useMemo(() => {
    return simulationData.find(node => getNodeKey(node) === active) || null;
  }, [simulationData, active]);

  // Get cluster index for the selected node (prefer semantic group_id from backend)
  const clusterIndex = useMemo(() => {
    if (!selectedCluster) return 0;
    return selectedCluster.group_id ?? selectedCluster.cluster ?? 0;
  }, [selectedCluster]);

  const handleSimulationUpdate = useCallback((newData: ConceptCluster[]) => {
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

  const client = useMemo(() => new QueryClient(), [])

  return (
    <QueryClientProvider client={client}>
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
        />
      </Layout>

      {/* Floating panel - outside Layout, positioned fixed */}
      {!isEmpty && (
        <FloatingPlanetPanel
          selectedCluster={selectedCluster}
          clusterIndex={clusterIndex}
          screenPositionRef={screenPositionRef}
          isAnimating={isAnimating}
          onClose={() => setActive("")}
        />
      )}

      {/* Empty state modal */}
      {isEmpty && (
        <EmptyStateModal
          onSimulationUpdate={handleSimulationUpdate}
          setLoadingState={setLoadingState}
        />
      )}
    </QueryClientProvider>
  )
}

export default App
