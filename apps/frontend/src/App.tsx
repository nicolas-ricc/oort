import Render from './cloud/Render'
import simulation from "./mocks/simulation.json"
import { useState } from 'react'
import { Menu } from './layout/Menu'
import { Layout } from './layout/Layout'

function App() {
  const concepts = simulation.map(({ _, concepts }) => concepts).flat()

  const [active, setActive] = useState(0)


  return (
    <Layout>
        <Render simulation={simulation} activeNode={active}/>
        <Menu concepts={concepts} onSelect={(id) => {
          console.log("clicked", id)
          setActive(id)}} />
    </Layout>
  )
}

export default App