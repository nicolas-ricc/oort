import { CommandDialog } from 'cmdk'
import Render from './cloud/Render'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator } from './components/ui/command'
import { Popover, PopoverContent } from '@radix-ui/react-popover'
import simulation from "./mocks/simulation.json"
import { AspectRatio } from '@radix-ui/react-aspect-ratio'
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