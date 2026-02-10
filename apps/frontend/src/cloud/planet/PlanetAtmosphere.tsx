import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { Mesh, ShaderMaterial } from 'three';
import { createAtmosphereMaterial } from '../shaders/AtmosphereShader';

type Props = {
  radius: number;
  color: string;
  isSelected: boolean;
};

export function PlanetAtmosphere({ radius, color, isSelected }: Props) {
  const meshRef = useRef<Mesh>(null);

  const material = useMemo(() => {
    return createAtmosphereMaterial(color, 0.6, 2.5);
  }, [color]);

  useFrame((state) => {
    if (meshRef.current) {
      const mat = meshRef.current.material as ShaderMaterial;
      if (mat.uniforms) {
        mat.uniforms.intensity.value = isSelected
          ? 0.8 + Math.sin(state.clock.elapsedTime * 3) * 0.2
          : 0.5;
      }
    }
  });

  return (
    <mesh ref={meshRef} scale={1.25}>
      <sphereGeometry args={[radius, 32, 32]} />
      <primitive object={material} attach="material" />
    </mesh>
  );
}
