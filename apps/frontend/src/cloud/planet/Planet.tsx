import { useMemo } from 'react';
import { PlanetMesh } from './PlanetMesh';
import { PlanetRings } from './PlanetRings';
import { PlanetSparkles } from './PlanetSparkles';
import { SelectionRing } from './SelectionRing';
import { getAtmosphereColor } from '../shaders/AtmosphereShader';

type PlanetProps = {
  position: number[];
  onClick: () => void;
  concepts: string[];
  isSelected: boolean;
  scaleSize?: number;
  clusterIndex?: number;
};

export function Planet({
  position,
  onClick,
  concepts,
  isSelected,
  scaleSize = 1,
  clusterIndex = 0
}: PlanetProps) {
  const atmosphereColor = useMemo(() => getAtmosphereColor(clusterIndex), [clusterIndex]);
  const hasRings = concepts.length >= 3;
  const baseRadius = 1.2 * scaleSize * Math.min(1 + concepts.length * 0.15, 2);

  // Deterministic seed from position so each planet gets unique terrain
  const seed = useMemo(() => {
    return position[0] * 500 + position[1] * 50 + position[2] * 5;
  }, [position]);

  return (
    <group position={position.map(p => parseFloat(String(p))) as [number, number, number]}>
      <PlanetMesh
        radius={baseRadius}
        clusterIndex={clusterIndex}
        atmosphereColor={atmosphereColor}
        isSelected={isSelected}
        onClick={onClick}
        seed={seed}
      />
      {hasRings && <PlanetRings radius={baseRadius} color={atmosphereColor} isSelected={isSelected} />}
      {isSelected && <SelectionRing radius={baseRadius} />}
      <PlanetSparkles radius={baseRadius} color={atmosphereColor} isSelected={isSelected} />
    </group>
  );
}
