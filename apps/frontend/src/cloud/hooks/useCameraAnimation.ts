import { useRef, useCallback, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';

function easeInOutQuad(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
}

type CameraTarget = {
  position: number[];
  lookAt: number[];
} | null;

export function useCameraAnimation(
  target: CameraTarget,
  duration = 1.4,
  onStart?: () => void,
  onComplete?: () => void
) {
  const { camera } = useThree();
  const [isAnimating, setIsAnimating] = useState(false);
  const startTime = useRef<number | null>(null);
  const startPosition = useRef(new THREE.Vector3());
  const startLookAt = useRef(new THREE.Vector3());
  const targetPosition = useRef(new THREE.Vector3());
  const targetLookAt = useRef(new THREE.Vector3());

  const startAnimation = useCallback(() => {
    if (!target) {
      return;
    }

    onStart?.();
    setIsAnimating(true);
    startPosition.current.copy(camera.position);

    const currentDir = new THREE.Vector3();
    camera.getWorldDirection(currentDir);
    startLookAt.current.addVectors(camera.position, currentDir.multiplyScalar(10));

    targetPosition.current.set(...(target.position as [number, number, number]));
    targetLookAt.current.set(...(target.lookAt as [number, number, number]));

    startTime.current = null;
  }, [target, camera, onStart]);

  useFrame((state) => {
    if (!isAnimating || !target) return;

    if (startTime.current === null) {
      startTime.current = state.clock.elapsedTime;
    }

    const elapsed = state.clock.elapsedTime - startTime.current;
    const progress = Math.min(elapsed / duration, 1);
    const eased = easeInOutQuad(progress);

    camera.position.lerpVectors(startPosition.current, targetPosition.current, eased);

    const currentLookAt = new THREE.Vector3();
    currentLookAt.lerpVectors(startLookAt.current, targetLookAt.current, eased);
    camera.lookAt(currentLookAt);

    if (progress >= 1) {
      setIsAnimating(false);
      startTime.current = null;
      camera.position.copy(targetPosition.current);
      camera.lookAt(targetLookAt.current);
      onComplete?.();
    }
  });

  return {
    isAnimating,
    startAnimation,
  };
}
