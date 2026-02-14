import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { useTexture } from '@react-three/drei';
import { Mesh, Color } from 'three';
import { useColorCorrectedMaterial } from '../hooks/useColorCorrectedMaterial';

type Props = {
  texturePath: string;
  radius: number;
  atmosphereColor: string;
  isSelected: boolean;
  onClick: () => void;
};

export function PlanetMesh({ texturePath, radius, atmosphereColor, isSelected, onClick }: Props) {
  const meshRef = useRef<Mesh>(null);
  const texture = useTexture(texturePath);

  // Create material with color correction baked in
  const material = useColorCorrectedMaterial(texture);

  // Set emissive properties (can't be in useMemo due to atmosphereColor dependency)
  material.emissive = new Color(atmosphereColor);
  material.emissiveIntensity = 0.05;

  useFrame(() => {
    if (meshRef.current) {
      meshRef.current.rotation.y += 0.0005;
    }
  });

  return (
    <mesh ref={meshRef} onClick={onClick} material={material}>
      <sphereGeometry args={[radius, 32, 32]} />
    </mesh>
  );
}
