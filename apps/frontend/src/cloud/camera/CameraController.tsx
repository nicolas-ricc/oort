import { useEffect, useRef } from 'react';
import { OrbitControls } from '@react-three/drei';
import { useCameraAnimation } from '../hooks/useCameraAnimation';
import { useLookAway } from '../hooks/useLookAway';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';

type CameraTarget = {
  position: number[];
  lookAt: number[];
} | null;

type Props = {
  target: CameraTarget;
  isLoading?: boolean;
  onAnimationStart?: () => void;
  onAnimationEnd?: () => void;
};

export function CameraController({ target, isLoading, onAnimationStart, onAnimationEnd }: Props) {
  const controlsRef = useRef<OrbitControlsImpl>(null);

  const { isAnimating, startAnimation } = useCameraAnimation(
    target,
    1.4,
    onAnimationStart,
    onAnimationEnd
  );

  // Look away during loading, but only if already viewing planets
  const shouldLookAway = !!isLoading && !!target;
  const { isLookingAway } = useLookAway(shouldLookAway);

  // Hide floating panel when look-away starts
  useEffect(() => {
    if (isLookingAway) {
      onAnimationStart?.();
    }
  }, [isLookingAway, onAnimationStart]);

  // Start planet-focus animation only when not loading.
  // During loading, target changes are queued — when isLoading flips to false
  // this effect re-fires and starts the animation from the looked-away orientation.
  useEffect(() => {
    if (target && !isLoading) {
      startAnimation();
    }
  }, [target, startAnimation, isLoading]);

  // When animation ends, update OrbitControls target to where camera is looking
  useEffect(() => {
    if (!isAnimating && controlsRef.current && target) {
      controlsRef.current.target.set(
        target.lookAt[0],
        target.lookAt[1],
        target.lookAt[2]
      );
      controlsRef.current.update();
    }
  }, [isAnimating, target]);

  return (
    <OrbitControls
      ref={controlsRef}
      makeDefault
      enableDamping
      dampingFactor={0.05}
      minDistance={5}
      maxDistance={500}
      enabled={!isAnimating && !isLookingAway}
    />
  );
}
