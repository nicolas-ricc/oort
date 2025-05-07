// App.js
import { Suspense, useRef, useMemo } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import { Scene } from "./Scene";
import { Bounds, OrbitControls, PerspectiveCamera, Trail } from "@react-three/drei";
import * as THREE from "three";
import { EffectComposer, Bloom } from '@react-three/postprocessing'


function ShootingStar() {
  const starRef = useRef();
  const particlesRef = useRef();
  const glowRef = useRef();
  
  // Create particles for the tail
  const particleCount = 50;
  const dummy = useMemo(() => new THREE.Object3D(), []);
  const particles = useMemo(() => {
    const temp = [];
    for (let i = 0; i < particleCount; i++) {
      temp.push({
        position: new THREE.Vector3(
          (Math.random() - 0.5) * 0.2,
          (Math.random() - 0.5) * 0.2,
          (Math.random() - 0.5) * 0.2
        ),
        scale: Math.random() * 0.5 + 0.5,
        velocity: new THREE.Vector3(
          (Math.random() - 0.5) * 0.01,
          (Math.random() - 0.5) * 0.01,
          (Math.random() - 0.5) * 0.01
        )
      });
    }
    return temp;
  }, []);
  
  useFrame((state) => {
    const t = state.clock.getElapsedTime() * 2;
    
    // Update main star position
    if (starRef.current) {
      starRef.current.position.set(
        Math.sin(t) * 4, 
        Math.atan(t) * Math.cos(t / 2) * 2, 
        Math.cos(t) * 4
      );
      
      // Update glow
      if (glowRef.current) {
        glowRef.current.position.copy(starRef.current.position);
        // Pulse the glow
        const pulseScale = 1 + Math.sin(t * 5) * 0.2;
        glowRef.current.scale.set(pulseScale, pulseScale, pulseScale);
      }
    }
    
    // Update particles
    if (particlesRef.current) {
      particles.forEach((particle, i) => {
        // Follow the star with some delay
        particle.position.x += particle.velocity.x;
        particle.position.y += particle.velocity.y;
        particle.position.z += particle.velocity.z;
        
        // Set the transform for this instance
        dummy.position.copy(starRef.current.position).add(particle.position);
        
        // Fade particles based on distance from main star
        const distance = dummy.position.distanceTo(starRef.current.position);
        const opacity = Math.max(0, 1 - distance * 5);
        const scale = particle.scale * opacity;
        
        dummy.scale.set(scale, scale, scale);
        dummy.updateMatrix();
        
        // Apply the matrix to the instanced item
        particlesRef.current.setMatrixAt(i, dummy.matrix);
      });
      
      particlesRef.current.instanceMatrix.needsUpdate = true;
    }
  });
  
  return (
    <group>
      {/* Main star */}
      <Trail 
        width={5} 
        length={15} 
        color={new THREE.Color(0.8, 0.2, 1)} 
        attenuation={(t) => t * t}
        decay={2}
      >
        <mesh ref={starRef}>
          <sphereGeometry args={[0.2]} />
          <meshBasicMaterial color={[5, 0.5, 5]} toneMapped={false} />
        </mesh>
      </Trail>
      
      {/* Glow effect */}
      <mesh ref={glowRef}>
        <sphereGeometry args={[0.5]} />
        <meshBasicMaterial 
          color={[2, 0.2, 2]} 
          transparent={true} 
          opacity={0.4} 
          toneMapped={false}
        />
      </mesh>
      
      {/* Particle system */}
      <instancedMesh ref={particlesRef} args={[null, null, particleCount]}>
        <sphereGeometry args={[0.05]} />
        <meshBasicMaterial 
          color={[3, 0.5, 3]} 
          transparent={true} 
          opacity={0.7} 
          toneMapped={false}
        />
      </instancedMesh>
    </group>
  );
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
  const vectors = (new THREE.Vector3(positions[2])).subVectors(positions[0], positions[1]);
  console.log("positions, vectors", positions, vectors)
  return (
    <Canvas className="w-full h-full" >
      <color attach="background" args={['#060605']} />
      <PerspectiveCamera makeDefault position={calculateCameraPosition(positions).position} ref={cameraRef} frustumCulled={false} near={0.05} far={1000} >
        {(_texture) =>
        (<>  <OrbitControls makeDefault />
          <Suspense fallback={null}>
            <Bounds clip observe margin={1.2} maxDuration={1} >
              <Scene
                nodes={simulation}
                activeNode={activeNode}
                setActive={setActive}
              />

            </Bounds>
              <EffectComposer>
                <Bloom 
                  intensity={0.1}
                  luminanceThreshold={0.2}
                  luminanceSmoothing={0.9}
                  width={200}
                />
              </EffectComposer>
          </Suspense>
        </>)
        }
      </PerspectiveCamera>

    </Canvas>
  );
}

export default Render;
