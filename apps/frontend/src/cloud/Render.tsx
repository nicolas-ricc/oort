// App.js
import { Suspense, useRef } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import { Scene } from "./Scene";
import { Bounds, OrbitControls, PerspectiveCamera, Trail } from "@react-three/drei";
import * as THREE from "three";
import { EffectComposer, Bloom } from '@react-three/postprocessing'


function ShootingStar() {
  const ref = useRef()
  useFrame((state) => {
    const t = state.clock.getElapsedTime() * 2
    ref.current.position.set(Math.sin(t) * 4, Math.atan(t) * Math.cos(t / 2) * 2, Math.cos(t) * 4)
  })
  return (
    <Trail width={5} length={8} color={new THREE.Color(2, 1, 10)} attenuation={(t) => t * t}>
      <mesh ref={ref}>
        <sphereGeometry args={[0.25]} />
        <meshBasicMaterial color={[10, 1, 10]} toneMapped={false} />
      </mesh>
    </Trail>
  )
}

function Render({ simulation, activeNode, setActive }) {

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
  const cameraRef = useRef(PerspectiveCamera.prototype);
  const positions = simulation.map(n => n.reduced_embedding.map(pos => parseFloat(pos)));

  return (
    <Canvas className="w-full h-full" >
      <color attach="background" args={['#060605']} />
      <PerspectiveCamera makeDefault position={calculateCameraPosition(positions).position} ref={cameraRef} >
        {(_texture) =>
        (<>  <OrbitControls makeDefault />
          <Suspense fallback={null}>
            <Bounds clip observe margin={1.2} maxDuration={1} >
              <ShootingStar />

              <Scene
                nodes={simulation}
                activeNode={activeNode}
                setActive={setActive}
              />

            </Bounds>

          </Suspense>
        </>)
        }
      </PerspectiveCamera>

    </Canvas>
  );
}

export default Render;
