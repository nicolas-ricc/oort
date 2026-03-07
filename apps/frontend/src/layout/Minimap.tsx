import { useRef, useEffect, useCallback } from 'react';
import * as THREE from 'three';
import { Simulation } from '@/App';
import { SCENE_SCALE } from '@/cloud/hooks/useSceneScale';
import { getAtmosphereColor } from '@/cloud/shaders/AtmosphereShader';

type Props = {
  simulationData: Simulation;
  shipPosition: THREE.Vector3 | null;
  shipYaw: number;
  activeNode: string;
};

const MAP_SIZE = 180;
const PADDING = 16;

function getNodeKey(node: { concepts: string[] }): string {
  return node.concepts.slice().sort().join('|');
}

export function Minimap({ simulationData, shipPosition, shipYaw, activeNode }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rafRef = useRef<number>(0);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = MAP_SIZE * dpr;
    canvas.height = MAP_SIZE * dpr;
    ctx.scale(dpr, dpr);

    // Clear
    ctx.clearRect(0, 0, MAP_SIZE, MAP_SIZE);
    ctx.fillStyle = 'rgba(24, 24, 27, 0.6)';
    ctx.fillRect(0, 0, MAP_SIZE, MAP_SIZE);

    if (simulationData.length === 0) return;

    // Compute planet positions (XZ plane)
    const positions = simulationData.map(node => {
      const emb = node.reduced_embedding;
      const x = (typeof emb[0] === 'string' ? parseFloat(emb[0]) : emb[0]) * SCENE_SCALE;
      const z = (typeof emb[2] === 'string' ? parseFloat(emb[2]) : emb[2]) * SCENE_SCALE;
      return { x, z, node };
    });

    // Compute bounds
    let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
    for (const p of positions) {
      minX = Math.min(minX, p.x);
      maxX = Math.max(maxX, p.x);
      minZ = Math.min(minZ, p.z);
      maxZ = Math.max(maxZ, p.z);
    }

    // Include ship position in bounds
    if (shipPosition) {
      minX = Math.min(minX, shipPosition.x);
      maxX = Math.max(maxX, shipPosition.x);
      minZ = Math.min(minZ, shipPosition.z);
      maxZ = Math.max(maxZ, shipPosition.z);
    }

    const rangeX = maxX - minX || 1;
    const rangeZ = maxZ - minZ || 1;
    const range = Math.max(rangeX, rangeZ) * 1.2;
    const centerX = (minX + maxX) / 2;
    const centerZ = (minZ + maxZ) / 2;

    const toMap = (wx: number, wz: number) => ({
      mx: PADDING + ((wx - centerX) / range + 0.5) * (MAP_SIZE - PADDING * 2),
      my: PADDING + ((wz - centerZ) / range + 0.5) * (MAP_SIZE - PADDING * 2),
    });

    // Faint grid
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.lineWidth = 0.5;
    for (let i = 0; i <= 4; i++) {
      const pos = PADDING + (i / 4) * (MAP_SIZE - PADDING * 2);
      ctx.beginPath();
      ctx.moveTo(pos, PADDING);
      ctx.lineTo(pos, MAP_SIZE - PADDING);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(PADDING, pos);
      ctx.lineTo(MAP_SIZE - PADDING, pos);
      ctx.stroke();
    }

    // Draw planet dots
    positions.forEach((p, idx) => {
      const { mx, my } = toMap(p.x, p.z);
      const nodeKey = getNodeKey(p.node);
      const isActive = nodeKey === activeNode;
      const color = getAtmosphereColor(idx % 30);

      ctx.beginPath();
      ctx.arc(mx, my, isActive ? 4 : 2, 0, Math.PI * 2);
      ctx.fillStyle = color;
      ctx.globalAlpha = isActive ? 1 : 0.6;
      ctx.fill();
      ctx.globalAlpha = 1;

      if (isActive) {
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    });

    // Draw ship position as white triangle
    if (shipPosition) {
      const { mx, my } = toMap(shipPosition.x, shipPosition.z);

      ctx.save();
      ctx.translate(mx, my);
      ctx.rotate(-shipYaw);

      ctx.beginPath();
      ctx.moveTo(0, -5);
      ctx.lineTo(-3, 4);
      ctx.lineTo(3, 4);
      ctx.closePath();
      ctx.fillStyle = '#ffffff';
      ctx.fill();
      ctx.restore();
    }

    // Border
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, MAP_SIZE, MAP_SIZE);
  }, [simulationData, shipPosition, shipYaw, activeNode]);

  useEffect(() => {
    let frameCount = 0;
    const loop = () => {
      frameCount++;
      if (frameCount % 4 === 0) { // ~15fps at 60fps
        draw();
      }
      rafRef.current = requestAnimationFrame(loop);
    };
    rafRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafRef.current);
  }, [draw]);

  return (
    <canvas
      ref={canvasRef}
      className="fixed bottom-6 left-4 z-30 pointer-events-none rounded"
      style={{ width: MAP_SIZE, height: MAP_SIZE }}
    />
  );
}
