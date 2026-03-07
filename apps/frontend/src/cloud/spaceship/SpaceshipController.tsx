import { useEffect, MutableRefObject } from 'react';
import * as THREE from 'three';
import { useSpaceshipControls, SpaceshipState } from './useSpaceshipControls';

type CameraTarget = {
  position: number[];
  lookAt: number[];
} | null;

type Props = {
  target: CameraTarget;
  isLoading?: boolean;
  onAnimationStart?: () => void;
  onAnimationEnd?: () => void;
  shipStateRef: MutableRefObject<SpaceshipState | null>;
  controlsEnabled: boolean;
};

export function SpaceshipController({
  target,
  isLoading,
  onAnimationStart,
  onAnimationEnd,
  shipStateRef,
  controlsEnabled,
}: Props) {
  const { stateRef, flyTo, isAutoFlying, maxSpeed } = useSpaceshipControls(controlsEnabled);

  // Expose state to parent
  useEffect(() => {
    shipStateRef.current = stateRef.current;
  });

  // Expose maxSpeed on the state ref for HUD access
  useEffect(() => {
    if (shipStateRef.current) {
      (shipStateRef.current as any).maxSpeed = maxSpeed;
    }
  }, [maxSpeed, shipStateRef]);

  // Trigger flyTo when target changes and not loading
  useEffect(() => {
    if (target && !isLoading) {
      onAnimationStart?.();
      const targetPos = new THREE.Vector3(
        target.position[0],
        target.position[1],
        target.position[2]
      );
      const lookAtPos = new THREE.Vector3(
        target.lookAt[0],
        target.lookAt[1],
        target.lookAt[2]
      );
      flyTo(targetPos, lookAtPos, 1.4);
    }
  }, [target, isLoading, flyTo, onAnimationStart]);

  // Fire animation end when auto-flight completes
  useEffect(() => {
    if (!isAutoFlying) {
      onAnimationEnd?.();
    }
  }, [isAutoFlying, onAnimationEnd]);

  return null;
}
