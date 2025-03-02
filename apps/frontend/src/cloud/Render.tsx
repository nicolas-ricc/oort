// App.js
import { Suspense } from "react";
import { Canvas } from "@react-three/fiber";
import { Scene } from "./Scene";

function Render({ simulation, activeNode }) {

  // Función para calcular el centro y la distancia óptima
  const calculateCameraPosition = (nodes) => {
    // Encuentra el centro de todos los nodos
    const center = nodes.reduce((acc, pos) => [
      acc[0] + pos[0] / nodes.length,
      acc[1] + pos[1] / nodes.length,
      acc[2] + pos[2] / nodes.length
    ], [0, 0, 0]) as [number, number, number];

    // Encuentra la distancia máxima desde el centro a cualquier nodo
    const maxDistance = Math.max(...nodes.map(pos =>
      Math.sqrt(
        Math.pow(pos[0] - center[0], 2) +
        Math.pow(pos[1] - center[1], 2) +
        Math.pow(pos[2] - center[2], 2)
      )
    ));

    // Añade un factor de margen para asegurar que todo sea visible
    const margin = 1.5;
    return {
      position: [
        center[0] + maxDistance * margin,
        center[1] + maxDistance * margin,
        center[2] + maxDistance * margin
      ] as [number, number, number],
      target: center
    };
  };
  const nodesAndConcepts = simulation.map(({ reduced_embedding, concepts }) => [reduced_embedding, concepts])
  const positions = nodesAndConcepts.map(n => n[0])
  return (
    <Canvas camera={{...calculateCameraPosition(positions)}} className="w-full h-full">

      <Suspense fallback={null}>
        <Scene
          nodes={nodesAndConcepts}
          activeNode={activeNode}
        />
      </Suspense>

    </Canvas>
  );
}

export default Render;
