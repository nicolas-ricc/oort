import { useEffect, useRef } from 'react';
import { OrbitControls } from '@react-three/drei';
import { useCameraAnimation } from '../hooks/useCameraAnimation';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';

type CameraTarget = {
  position: number[];
  lookAt: number[];
} | null;

type Props = {
  target: CameraTarget;
  onAnimationStart?: () => void;
  onAnimationEnd?: () => void;
};

export function CameraController({ target, onAnimationStart, onAnimationEnd }: Props) {
  const controlsRef = useRef<OrbitControlsImpl>(null);
  const { isAnimating, startAnimation } = useCameraAnimation(
    target,
    1.4,
    onAnimationStart,
    onAnimationEnd
  );

  useEffect(() => {
    if (target) {
      startAnimation();
    }
  }, [target, startAnimation]);

  useEffect(() => {
    // When animation ends, update OrbitControls target to where camera is looking
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
      enabled={!isAnimating}
    />
  );
}
