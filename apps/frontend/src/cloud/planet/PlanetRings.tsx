import { Ring } from '@react-three/drei';

type Props = {
  radius: number;
  color: string;
  isSelected: boolean;
};

export function PlanetRings({ radius, color, isSelected }: Props) {
  return (
    <group rotation={[Math.PI / 4, 0, Math.PI / 6]}>
      <Ring args={[radius * 1.4, radius * 1.8, 64]}>
        <meshBasicMaterial
          color={color}
          transparent
          opacity={isSelected ? 0.6 : 0.3}
          side={2}
        />
      </Ring>
      {/* Secondary ring */}
      <Ring args={[radius * 1.9, radius * 2.1, 64]}>
        <meshBasicMaterial
          color={color}
          transparent
          opacity={isSelected ? 0.3 : 0.15}
          side={2}
        />
      </Ring>
    </group>
  );
}
