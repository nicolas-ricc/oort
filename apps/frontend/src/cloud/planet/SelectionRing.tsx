import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { Mesh } from 'three';

type Props = {
  radius: number;
};

export function SelectionRing({ radius }: Props) {
  const ringRef = useRef<Mesh>(null);

  useFrame((_, delta) => {
    if (ringRef.current) {
      ringRef.current.rotation.z += delta * 0.5;
    }
  });

  return (
    <mesh ref={ringRef} rotation={[Math.PI / 2, 0, 0]}>
      <ringGeometry args={[radius * 1.6, radius * 1.7, 64]} />
      <meshBasicMaterial
        color="#86efac"
        transparent
        opacity={0.5}
        side={2}
      />
    </mesh>
  );
}
