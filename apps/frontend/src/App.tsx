import Render from './cloud/Render'
import simulation from "./mocks/simulation.json"
import { useCallback, useMemo, useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

export type ConceptCluster = {
  concepts: string[];
  reduced_embedding: number[];
  cluster: number;
}

export type Simulation = ConceptCluster[]

function App() {
  const [simulationData, setSimulationData] = useState<Simulation>(simulation)
  const [active, setActive] = useState<string>(simulationData[0]?.reduced_embedding.join(""))
  const [isLoading, setIsLoading] = useState(false)

  useMemo(() => {
    console.log(simulationData)
    console.log(active)
  }, [active])

  const handleSimulationUpdate = useCallback((newData) => {
    setSimulationData(newData)
    setActive(newData[0]?.reduced_embedding.join(""))
  }, [])

  // This function will be passed to Menu to set the loading state
  const setLoadingState = useCallback((loading: boolean) => {
    setIsLoading(loading)
  }, [])

  const client = new QueryClient()
  return (
    <QueryClientProvider client={client}>
        <Layout>
          <Render simulation={simulationData} activeNode={active} setActive={setActive} />
          <Menu concepts={simulationData}
            onSelect={(concept) => {
              setActive((simulationData.find(s => s.concepts.includes(concept)) || simulationData[0]).reduced_embedding.join(""))
            }}
            onSimulationUpdate={handleSimulationUpdate}
            active={active}
            setLoadingState={setLoadingState}
          />
        </Layout>
    </QueryClientProvider>
  )
}

export default App