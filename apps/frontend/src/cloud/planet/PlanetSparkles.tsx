import { Sparkles } from '@react-three/drei';

type Props = {
  radius: number;
  color: string;
  isSelected: boolean;
};

export function PlanetSparkles({ radius, color, isSelected }: Props) {
  return (
    <>
      <Sparkles
        position={[0, 0, 0]}
        count={isSelected ? 60 : 30}
        scale={isSelected ? radius * 4 : radius * 3}
        size={isSelected ? 3 : 2}
        speed={isSelected ? 1.5 : 0.5}
        opacity={isSelected ? 0.9 : 0.5}
        color={color}
      />
      {/* Orbital particles when selected */}
      {isSelected && (
        <Sparkles
          position={[0, 0, 0]}
          count={40}
          scale={radius * 5}
          size={1.5}
          speed={2}
          opacity={0.7}
          color="#ffffff"
        />
      )}
    </>
  );
}
