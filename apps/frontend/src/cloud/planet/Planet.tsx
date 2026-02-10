import { useMemo } from 'react';
import { PlanetMesh } from './PlanetMesh';
import { PlanetRings } from './PlanetRings';
import { PlanetLabel } from './PlanetLabel';
import { PlanetSparkles } from './PlanetSparkles';
import { SelectionRing } from './SelectionRing';
import { getAtmosphereColor } from '../shaders/AtmosphereShader';

type PlanetProps = {
  texturePath: string;
  position: number[];
  onClick: () => void;
  concepts: string[];
  isSelected: boolean;
  shouldShowConcepts?: boolean;
  scaleSize?: number;
  clusterIndex?: number;
};

export function Planet({
  texturePath,
  position,
  onClick,
  concepts,
  isSelected,
  shouldShowConcepts = true,
  scaleSize = 1,
  clusterIndex = 0
}: PlanetProps) {
  const atmosphereColor = useMemo(() => getAtmosphereColor(clusterIndex), [clusterIndex]);
  const hasRings = concepts.length >= 3;
  const baseRadius = 1.2 * scaleSize * Math.min(1 + concepts.length * 0.15, 2);

  return (
    <group position={position.map(p => parseFloat(String(p))) as [number, number, number]}>
      <PlanetMesh
        texturePath={texturePath}
        radius={baseRadius}
        atmosphereColor={atmosphereColor}
        isSelected={isSelected}
        onClick={onClick}
      />
      {hasRings && <PlanetRings radius={baseRadius} color={atmosphereColor} isSelected={isSelected} />}
      {isSelected && <SelectionRing radius={baseRadius} />}
      {shouldShowConcepts && <PlanetLabel concepts={concepts} radius={baseRadius} isSelected={isSelected} />}
      <PlanetSparkles radius={baseRadius} color={atmosphereColor} isSelected={isSelected} />
    </group>
  );
}
