import { Text, Billboard } from '@react-three/drei';

type Props = {
  concepts: string[];
  radius: number;
  isSelected: boolean;
};

export function PlanetLabel({ concepts, radius, isSelected }: Props) {
  return (
    <Billboard follow={true} lockX={false} lockY={false} lockZ={false}>
      <group position={[0, radius + 1.5, 0]}>
        {/* Background panel */}
        <mesh position={[0, concepts.length * 0.2, -0.1]}>
          <planeGeometry args={[
            Math.max(...concepts.map(c => c.length * 0.12), 2) + 0.5,
            concepts.length * 0.5 + 0.4
          ]} />
          <meshBasicMaterial
            color="#000000"
            transparent
            opacity={isSelected ? 0.8 : 0.4}
          />
        </mesh>

        {/* Concept labels */}
        {concepts.map((concept, idx) => (
          <Text
            key={concept}
            position={[0, (concepts.length - 1 - idx) * 0.4, 0]}
            fontSize={0.25}
            color={isSelected ? "#ffffff" : "#88ffaa"}
            anchorX="center"
            anchorY="middle"
            outlineWidth={0.02}
            outlineColor="#000000"
          >
            {concept}
          </Text>
        ))}
      </group>
    </Billboard>
  );
}
