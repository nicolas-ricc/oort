import { useRef, useEffect, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/**
 * Smoothly rotates the camera ~90° to the side (away from planets) when active.
 * Position stays fixed — only the viewing direction changes.
 */
export function useLookAway(active: boolean, duration = 1.5) {
  const { camera } = useThree();
  const [isLookingAway, setIsLookingAway] = useState(false);
  const animatingRef = useRef(false);
  const startTime = useRef<number | null>(null);
  const startLookAt = useRef(new THREE.Vector3());
  const targetLookAt = useRef(new THREE.Vector3());

  useEffect(() => {
    if (active) {
      const currentDir = new THREE.Vector3();
      camera.getWorldDirection(currentDir);

      startLookAt.current.copy(camera.position).addScaledVector(currentDir, 10);

      // Compute a direction ~90° to the right of the current view
      const up = new THREE.Vector3(0, 1, 0);
      const sideDir = new THREE.Vector3().crossVectors(currentDir, up);

      if (sideDir.lengthSq() < 0.001) {
        // Camera looking straight up/down — use world X as fallback
        sideDir.set(1, 0, 0);
      }
      sideDir.normalize();

      // Add upward tilt for a gentle "drifting into space" feel
      sideDir.y += 0.4;
      sideDir.normalize();

      targetLookAt.current.copy(camera.position).addScaledVector(sideDir, 10);

      animatingRef.current = true;
      setIsLookingAway(true);
      startTime.current = null;
    } else {
      animatingRef.current = false;
      setIsLookingAway(false);
    }
  }, [active, camera]);

  useFrame((state) => {
    if (!animatingRef.current) return;

    if (startTime.current === null) {
      startTime.current = state.clock.elapsedTime;
    }

    const elapsed = state.clock.elapsedTime - startTime.current;
    const progress = Math.min(elapsed / duration, 1);
    const eased = easeInOutCubic(progress);

    const currentLookAt = new THREE.Vector3();
    currentLookAt.lerpVectors(startLookAt.current, targetLookAt.current, eased);
    camera.lookAt(currentLookAt);

    if (progress >= 1) {
      animatingRef.current = false;
    }
  });

  return { isLookingAway };
}
