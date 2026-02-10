import Render from './cloud/Render'
import simulation from "./mocks/simulation.json"
import { useCallback, useMemo, useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'
import { ConceptPanel } from './layout/ConceptPanel'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useNavigation } from './hooks/useNavigation'

// Must match SCENE_SCALE in Scene.tsx and Render.tsx
const SCENE_SCALE = 2;

export type ConceptCluster = {
  concepts: string[];
  reduced_embedding: number[];
  cluster: number;
}

export type Simulation = ConceptCluster[]

function App() {
  const [simulationData, setSimulationData] = useState<Simulation>(simulation)
  const [active, setActive] = useState<string>(() => {
    if (simulationData[0]?.reduced_embedding) {
      const safeEmbedding = simulationData[0].reduced_embedding.map(p => {
        let val = typeof p === 'string' ? parseFloat(p) : Number(p);
        return isNaN(val) ? 0 : val * SCENE_SCALE;
      });
      return safeEmbedding.map(String).join("-");
    }
    return "";
  })
  const [isLoading, setIsLoading] = useState(false)

  // Navigation hook
  const navigation = useNavigation(simulationData, active, setActive);

  // Get the currently selected cluster
  const selectedCluster = useMemo(() => {
    return simulationData.find(node => {
      if (!node?.reduced_embedding) return false;
      const safeEmbedding = node.reduced_embedding.map(p => {
        let val = typeof p === 'string' ? parseFloat(p) : Number(p);
        return isNaN(val) ? 0 : val * SCENE_SCALE;
      });
      return safeEmbedding.map(String).join("-") === active;
    }) || null;
  }, [simulationData, active]);

  // Get cluster index for the selected node
  const clusterIndex = useMemo(() => {
    if (!selectedCluster) return 0;
    return selectedCluster.cluster || 0;
  }, [selectedCluster]);

  const handleSimulationUpdate = useCallback((newData: ConceptCluster[]) => {
    setSimulationData(prev => {
      const existingKeys = new Set(prev.map(node => {
        if (!node?.reduced_embedding) return null;
        const safeEmbedding = node.reduced_embedding.map(p => {
          let val = typeof p === 'string' ? parseFloat(p) : Number(p);
          return isNaN(val) ? 0 : val;
        });
        return safeEmbedding.map(String).join("-");
      }).filter(Boolean));

      const uniqueNewData = newData.filter(node => {
        if (!node?.reduced_embedding) return false;
        const safeEmbedding = node.reduced_embedding.map(p => {
          let val = typeof p === 'string' ? parseFloat(p) : Number(p);
          return isNaN(val) ? 0 : val;
        });
        const key = safeEmbedding.map(String).join("-");
        return !existingKeys.has(key);
      });

      return [...prev, ...uniqueNewData];
    });
    if (newData[0]?.reduced_embedding) {
      const safeEmbedding = newData[0].reduced_embedding.map(p => {
        let val = typeof p === 'string' ? parseFloat(p) : Number(p);
        return isNaN(val) ? 0 : val * SCENE_SCALE;
      });
      setActive(safeEmbedding.map(String).join("-"));
    }
  }, [])

  const setLoadingState = useCallback((loading: boolean) => {
    setIsLoading(loading)
  }, [])

  const client = useMemo(() => new QueryClient(), [])

  return (
    <QueryClientProvider client={client}>
      <Layout>
        <Render
          simulation={simulationData}
          activeNode={active}
          setActive={setActive}
          onNavigateNext={navigation.navigateToNext}
          onNavigatePrevious={navigation.navigateToPrevious}
          onToggleTour={navigation.toggleTour}
          onResetToOverview={navigation.resetToOverview}
          onNavigateToIndex={navigation.navigateToIndex}
        />
        <Menu
          concepts={simulationData}
          onSelect={(concept) => {
            const foundNode = simulationData.find(s => s.concepts.includes(concept)) || simulationData[0];
            if (foundNode?.reduced_embedding) {
              const safeEmbedding = foundNode.reduced_embedding.map(p => {
                let val = typeof p === 'string' ? parseFloat(p) : Number(p);
                return isNaN(val) ? 0 : val * SCENE_SCALE;
              });
              setActive(safeEmbedding.map(String).join("-"));
            }
          }}
          onSimulationUpdate={handleSimulationUpdate}
          active={active}
          setLoadingState={setLoadingState}
        />
        <ConceptPanel
          selectedCluster={selectedCluster}
          currentIndex={navigation.currentIndex}
          totalCount={navigation.totalCount}
          tourMode={navigation.tourMode}
          clusterIndex={clusterIndex}
          onNavigateNext={navigation.navigateToNext}
          onNavigatePrevious={navigation.navigateToPrevious}
          onToggleTour={navigation.toggleTour}
          onNavigateToIndex={navigation.navigateToIndex}
        />
      </Layout>
    </QueryClientProvider>
  )
}

export default App
