// App.js
import React, { useRef, Suspense } from "react";
import { Canvas, useFrame, useLoader } from "@react-three/fiber";
import { OrbitControls, Stars } from "@react-three/drei";
import * as THREE from "three";

function Planet({ texturePath, position, scale, orbitSpeed, rotationSpeed }) {
  const planetRef = useRef();
  const texture = useLoader(THREE.TextureLoader, texturePath);

  // Orbit & rotation animation
  useFrame(({ clock }) => {
    planetRef.current.rotation.y += rotationSpeed; // Rotation around its own axis
  });

  return (
    <>
    <mesh ref={planetRef}>
      <sphereGeometry args={scale} />
      <meshStandardMaterial map={texture} />
    </mesh>
    </>
  );
}

function Scene() {
  return (
    <>
      <ambientLight intensity={0.2} />
      <pointLight position={[0, 0, 0]} intensity={2} />
      <Stars radius={100} depth={50} count={5000} factor={7} />


      {/* Planets with Textures */}
      <Planet
        texturePath="https://upload.wikimedia.org/wikipedia/commons/9/97/The_Earth_seen_from_Apollo_17.jpg" // Earth
        position={[6, 0, 0]}
        scale={[3, 32, 32]}
        orbitSpeed={0.2}
        rotationSpeed={0.0005}
      />
      <Planet
        texturePath="https://upload.wikimedia.org/wikipedia/commons/e/e2/Jupiter.jpg" // Earth
        position={[9, 0, 0]}
        scale={[0.8, 32, 32]}
        orbitSpeed={0.1}
        rotationSpeed={0.005}
      />
      <Planet
        texturePath="https://upload.wikimedia.org/wikipedia/commons/e/e2/Jupiter.jpg" // Earth
        position={[12, 0, 0]}
        scale={[1.5, 32, 32]}
        orbitSpeed={0.15}
        rotationSpeed={0.005}
      />
    </>
  );
}

function Graph() {
  return (
    <Canvas camera={{ position: [0, 10, 20], fov: 60 }}>
      <Suspense fallback={null}>
        <OrbitControls enableZoom={true} />
        <Scene />
      </Suspense>
    </Canvas>
  );
}

export default Graph;
