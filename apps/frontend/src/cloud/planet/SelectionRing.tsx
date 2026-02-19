import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

type Props = {
  radius: number;
};

function createChevronShape(size: number): THREE.Shape {
  const shape = new THREE.Shape();
  const w = size * 0.35;
  const h = size;

  // Chevron pointing inward (right-pointing bracket: >)
  shape.moveTo(0, h / 2);
  shape.lineTo(w, 0);
  shape.lineTo(0, -h / 2);
  shape.lineTo(-w * 0.4, -h / 2 + h * 0.15);
  shape.lineTo(w * 0.3, 0);
  shape.lineTo(-w * 0.4, h / 2 - h * 0.15);
  shape.closePath();

  return shape;
}

export function SelectionRing({ radius }: Props) {
  const groupRef = useRef<THREE.Group>(null);
  const tipsRef = useRef<THREE.Group[]>([]);

  const chevronShape = useMemo(() => createChevronShape(radius * 0.4), [radius]);
  const shapeGeometry = useMemo(() => new THREE.ShapeGeometry(chevronShape), [chevronShape]);

  // Cardinal rotations: right, top, left, bottom
  const cardinalAngles = useMemo(() => [0, Math.PI / 2, Math.PI, (3 * Math.PI) / 2], []);

  useFrame((state, delta) => {
    if (!groupRef.current) return;

    // Slow rotation around Y axis
    groupRef.current.rotation.y += delta * 0.3;

    // Pulsing inward/outward
    const t = state.clock.getElapsedTime();
    const pulse = radius * 1.8 + Math.sin(t * 2) * radius * 0.2;

    tipsRef.current.forEach((tip, i) => {
      if (!tip) return;
      const angle = cardinalAngles[i];
      tip.position.x = Math.cos(angle) * pulse;
      tip.position.y = Math.sin(angle) * pulse;
    });
  });

  return (
    <group ref={groupRef}>
      {/* 4 chevron tips at cardinal positions */}
      {cardinalAngles.map((angle, i) => (
        <group
          key={i}
          ref={(el) => { if (el) tipsRef.current[i] = el; }}
          position={[Math.cos(angle) * radius * 1.8, Math.sin(angle) * radius * 1.8, 0]}
          rotation={[0, 0, angle - Math.PI]}
        >
          <mesh geometry={shapeGeometry}>
            <meshBasicMaterial
              color="#4ade80"
              transparent
              opacity={0.7}
              side={THREE.DoubleSide}
              toneMapped={false}
              blending={THREE.AdditiveBlending}
            />
          </mesh>
        </group>
      ))}

    </group>
  );
}
