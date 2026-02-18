import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

const STREAK_COUNT = 150;
const BOUNDS = 40;
const MIN_SPEED = 8;
const MAX_SPEED = 20;
const MIN_LENGTH = 0.5;
const MAX_LENGTH = 1.5;

// Direction: diagonal down-left with slight depth
const DIR = new THREE.Vector3(-1, -1, -0.3).normalize();

type Streak = {
  x: number;
  y: number;
  z: number;
  speed: number;
  length: number;
};

function createStreak(): Streak {
  return {
    x: (Math.random() - 0.5) * BOUNDS * 2,
    y: (Math.random() - 0.5) * BOUNDS * 2,
    z: (Math.random() - 0.5) * BOUNDS * 2,
    speed: MIN_SPEED + Math.random() * (MAX_SPEED - MIN_SPEED),
    length: MIN_LENGTH + Math.random() * (MAX_LENGTH - MIN_LENGTH),
  };
}

export function ShootingStarsRain() {
  const lineRef = useRef<THREE.LineSegments>(null);

  const { streaks, positions } = useMemo(() => {
    const s = Array.from({ length: STREAK_COUNT }, createStreak);
    // 2 vertices per streak, 3 floats per vertex
    const p = new Float32Array(STREAK_COUNT * 2 * 3);
    return { streaks: s, positions: p };
  }, []);

  const geom = useMemo(() => {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    return g;
  }, [positions]);

  useFrame((_, delta) => {
    const dt = Math.min(delta, 0.05); // clamp to avoid huge jumps

    for (let i = 0; i < STREAK_COUNT; i++) {
      const s = streaks[i];

      // Move along direction
      s.x += DIR.x * s.speed * dt;
      s.y += DIR.y * s.speed * dt;
      s.z += DIR.z * s.speed * dt;

      // Wrap around when out of bounds
      if (s.x < -BOUNDS || s.y < -BOUNDS || s.z < -BOUNDS) {
        s.x = (Math.random() - 0.5) * BOUNDS * 2;
        s.y = BOUNDS + Math.random() * 5;
        s.z = (Math.random() - 0.5) * BOUNDS * 2;
      }

      const idx = i * 6; // 2 vertices * 3 components

      // Tail vertex (behind the streak)
      positions[idx] = s.x - DIR.x * s.length;
      positions[idx + 1] = s.y - DIR.y * s.length;
      positions[idx + 2] = s.z - DIR.z * s.length;

      // Head vertex
      positions[idx + 3] = s.x;
      positions[idx + 4] = s.y;
      positions[idx + 5] = s.z;
    }

    const attr = geom.getAttribute('position') as THREE.BufferAttribute;
    attr.needsUpdate = true;
  });

  return (
    <lineSegments ref={lineRef} geometry={geom} frustumCulled={false}>
      <lineBasicMaterial
        color="#ffffff"
        transparent
        opacity={0.4}
        toneMapped={false}
      />
    </lineSegments>
  );
}
