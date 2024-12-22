import './App.css'
import Graph from './cloud/render'
function App() {

  return (
    <div style={{ display: "grid", gridTemplateRows: "10% 90%", gridTemplateColumns: "100%", width: "100%" }}>
      <h1>Force Simulation</h1>
      <Graph  />
    </div>
  )
}

export default App
