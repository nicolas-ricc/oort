import Render from './cloud/Render'
import simulation from "./mocks/simulation.json"
import { useCallback, useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

type Simulation = {
  concepts: string[];
  reduced_embedding: number[];
  cluster: number;
}[]

function App() {
  const [simulationData, setSimulationData] = useState<Simulation>(simulation)
  const [active, setActive] = useState<number>(0)

  const concepts = simulationData?.map(({ concepts }) => concepts).flat() || []

  
  const handleSimulationUpdate = useCallback((newData) => {
    console.log("NEW DATA", newData)
    setSimulationData(newData)
    setActive(0)
  }, [])
  const client = new QueryClient()
  return ( <QueryClientProvider client={client}>
    <Layout>
        <Render simulation={simulationData} activeNode={active}/>
        <Menu concepts={concepts} onSelect={(id) => {
          console.log("clicked", id)
          setActive(id)}}
          onSimulationUpdate={handleSimulationUpdate}
          activeIndex={active}
          setActiveIndex={setActive}
          />
    </Layout>
    </QueryClientProvider>
  )
}

export default App