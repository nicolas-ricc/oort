import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { nebulaVertexShader, nebulaFragmentShader } from '../shaders/NebulaShader';

type ClusterData = {
  centroid: number[];
  radius: number;
  color: string;
};

type Props = {
  clusters: ClusterData[];
};

const QUADS_PER_CLUSTER = 20;

export function NebulaClouds({ clusters }: Props) {
  const meshRef = useRef<THREE.Mesh>(null);

  const { geometry, material } = useMemo(() => {
    const totalQuads = clusters.length * QUADS_PER_CLUSTER;
    if (totalQuads === 0) {
      return {
        geometry: new THREE.BufferGeometry(),
        material: new THREE.ShaderMaterial(),
      };
    }

    // Base quad (plane) — merged into instanced positions
    const positions: number[] = [];
    const uvs: number[] = [];
    const indices: number[] = [];
    const scales: number[] = [];
    const seeds: number[] = [];
    const colors: number[] = [];

    let vertexOffset = 0;
    for (let ci = 0; ci < clusters.length; ci++) {
      const cluster = clusters[ci];
      const clusterColor = new THREE.Color(cluster.color);

      for (let qi = 0; qi < QUADS_PER_CLUSTER; qi++) {
        // Random position within cluster bounding sphere
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos(2 * Math.random() - 1);
        const r = cluster.radius * (0.3 + Math.random() * 0.7);

        const cx = cluster.centroid[0] + r * Math.sin(phi) * Math.cos(theta);
        const cy = cluster.centroid[1] + r * Math.sin(phi) * Math.sin(theta);
        const cz = cluster.centroid[2] + r * Math.cos(phi);

        const scale = 2 + Math.random() * 4;
        const seed = Math.random() * 100;

        // 4 vertices for a quad
        // Billboarding handled in vertex shader, but we place them at the world pos
        const halfScale = scale;
        const quadVerts = [
          [cx - halfScale, cy - halfScale, cz],
          [cx + halfScale, cy - halfScale, cz],
          [cx + halfScale, cy + halfScale, cz],
          [cx - halfScale, cy + halfScale, cz],
        ];

        for (const v of quadVerts) {
          positions.push(v[0], v[1], v[2]);
          scales.push(scale);
          seeds.push(seed);
          colors.push(clusterColor.r, clusterColor.g, clusterColor.b);
        }

        uvs.push(0, 0, 1, 0, 1, 1, 0, 1);

        indices.push(
          vertexOffset, vertexOffset + 1, vertexOffset + 2,
          vertexOffset, vertexOffset + 2, vertexOffset + 3
        );
        vertexOffset += 4;
      }
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    geo.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
    geo.setAttribute('aScale', new THREE.Float32BufferAttribute(scales, 1));
    geo.setAttribute('aSeed', new THREE.Float32BufferAttribute(seeds, 1));
    geo.setAttribute('aColor', new THREE.Float32BufferAttribute(colors, 3));
    geo.setIndex(indices);

    const mat = new THREE.ShaderMaterial({
      vertexShader: nebulaVertexShader,
      fragmentShader: nebulaFragmentShader,
      uniforms: {
        uTime: { value: 0 },
      },
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      side: THREE.DoubleSide,
    });

    return { geometry: geo, material: mat };
  }, [clusters]);

  useFrame((_state, delta) => {
    if (material instanceof THREE.ShaderMaterial && material.uniforms.uTime) {
      material.uniforms.uTime.value += delta;
    }
  });

  if (clusters.length === 0) return null;

  return (
    <mesh ref={meshRef} geometry={geometry} material={material} frustumCulled={false} />
  );
}
