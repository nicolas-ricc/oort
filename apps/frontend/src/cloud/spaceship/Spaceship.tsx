import { useRef, useEffect } from 'react';
import { useThree } from '@react-three/fiber';
import { Trail } from '@react-three/drei';
import * as THREE from 'three';
import { SpaceshipState } from './useSpaceshipControls';
import { MutableRefObject } from 'react';

type Props = {
  shipStateRef: MutableRefObject<SpaceshipState | null>;
  visible?: boolean;
};

export function Spaceship({ shipStateRef, visible = true }: Props) {
  const { camera } = useThree();
  const groupRef = useRef<THREE.Group>(null);
  const trailMeshRef = useRef<THREE.Mesh>(null);

  // Attach spaceship mesh to camera
  useEffect(() => {
    const group = groupRef.current;
    if (!group) return;
    camera.add(group);
    return () => {
      camera.remove(group);
    };
  }, [camera]);

  const isBoosting = shipStateRef.current?.isBoosting ?? false;
  const trailColor = isBoosting
    ? new THREE.Color(0.4, 0.6, 1.0)
    : new THREE.Color(0.2, 0.3, 0.6);

  return (
    <>
      {/* Ship hull - attached to camera */}
      <group ref={groupRef} visible={visible}>
        {/* Main body - cone pointing forward */}
        <mesh position={[0, -0.3, -1.5]} rotation={[Math.PI / 2, 0, 0]}>
          <coneGeometry args={[0.08, 0.3, 6]} />
          <meshBasicMaterial color="#8899aa" />
        </mesh>
        {/* Cockpit window */}
        <mesh position={[0, -0.27, -1.4]}>
          <boxGeometry args={[0.06, 0.03, 0.08]} />
          <meshBasicMaterial color="#44ddff" toneMapped={false} />
        </mesh>
        {/* Left wing */}
        <mesh position={[-0.12, -0.32, -1.35]} rotation={[0, 0, -0.3]}>
          <boxGeometry args={[0.12, 0.01, 0.15]} />
          <meshBasicMaterial color="#667788" />
        </mesh>
        {/* Right wing */}
        <mesh position={[0.12, -0.32, -1.35]} rotation={[0, 0, 0.3]}>
          <boxGeometry args={[0.12, 0.01, 0.15]} />
          <meshBasicMaterial color="#667788" />
        </mesh>
      </group>

      {/* Engine trail - needs to be in world space for Trail to work */}
      <Trail
        width={3}
        length={10}
        color={trailColor}
        attenuation={(t) => t * t}
        decay={2}
      >
        <mesh ref={trailMeshRef} visible={false}>
          <sphereGeometry args={[0.01]} />
          <meshBasicMaterial />
        </mesh>
      </Trail>
    </>
  );
}
