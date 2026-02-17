import { Suspense, useRef, useState, useCallback, MutableRefObject } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import { Scene, ColorClusterInfo } from "./Scene";
import { PerspectiveCamera, Trail } from "@react-three/drei";
import * as THREE from "three";
import { StarField } from "./background/StarField";
import { PostProcessing } from "./effects/PostProcessing";
import { CameraController } from "./camera/CameraController";
import { SCENE_SCALE } from "./hooks/useSceneScale";

function LoadingIndicator() {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (meshRef.current) {
      meshRef.current.rotation.y = state.clock.elapsedTime;
    }
  });

  return (
    <mesh ref={meshRef}>
      <icosahedronGeometry args={[2, 1]} />
      <meshBasicMaterial color="#666666" wireframe />
    </mesh>
  );
}

function ShootingStar() {
  const starRef = useRef<THREE.Mesh>(null);
  const glowRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    const t = state.clock.getElapsedTime() * 2;

    if (starRef.current) {
      starRef.current.position.set(
        Math.sin(t) * 4,
        Math.atan(t) * Math.cos(t / 2) * 2,
        Math.cos(t) * 4
      );

      if (glowRef.current) {
        glowRef.current.position.copy(starRef.current.position);
        const pulseScale = 1 + Math.sin(t * 5) * 0.2;
        glowRef.current.scale.set(pulseScale, pulseScale, pulseScale);
      }
    }
  });

  return (
    <group>
      <Trail
        width={5}
        length={15}
        color={new THREE.Color(0.8, 0.2, 1)}
        attenuation={(t) => t * t}
        decay={2}
      >
        <mesh ref={starRef}>
          <sphereGeometry args={[0.2]} />
          <meshBasicMaterial color={[5, 0.5, 5]} toneMapped={false} />
        </mesh>
      </Trail>

      <mesh ref={glowRef}>
        <sphereGeometry args={[0.5]} />
        <meshBasicMaterial
          color={[2, 0.2, 2]}
          transparent={true}
          opacity={0.4}
          toneMapped={false}
        />
      </mesh>
    </group>
  );
}

type RenderProps = {
  simulation: any[];
  activeNode: string;
  setActive: (key: string) => void;
  onNavigateNext?: () => void;
  onNavigatePrevious?: () => void;
  onToggleTour?: () => void;
  onResetToOverview?: () => void;
  onNavigateToIndex?: (index: number) => void;
  screenPositionRef?: MutableRefObject<{ x: number; y: number } | null>;
  onAnimatingChange?: (animating: boolean) => void;
  onColorClusterInfo?: (info: ColorClusterInfo | null) => void;
};

function Render({
  simulation,
  activeNode,
  setActive,
  onNavigateNext,
  onNavigatePrevious,
  onToggleTour,
  onResetToOverview,
  onNavigateToIndex,
  screenPositionRef,
  onAnimatingChange,
  onColorClusterInfo
}: RenderProps) {
  const [isAnimating, setIsAnimating] = useState(false);
  const [cameraTarget, setCameraTarget] = useState<{ position: number[]; lookAt: number[] } | null>(null);

  const handleAnimationStart = useCallback(() => {
    setIsAnimating(true);
    onAnimatingChange?.(true);
  }, [onAnimatingChange]);

  const handleAnimationEnd = useCallback(() => {
    setIsAnimating(false);
    onAnimatingChange?.(false);
  }, [onAnimatingChange]);

  const handleCameraTargetChange = useCallback((target: { position: number[]; lookAt: number[] } | null) => {
    setCameraTarget(target);
  }, []);

  const calculateCameraPosition = (nodes: number[][]) => {
    if (!nodes || nodes.length === 0) {
      return {
        position: [10, 10, 10] as [number, number, number],
        target: [0, 0, 0] as [number, number, number]
      };
    }

    const center = nodes.reduce((acc, pos) => [
      acc[0] + pos[0] / nodes.length,
      acc[1] + pos[1] / nodes.length,
      acc[2] + pos[2] / nodes.length
    ], [0, 0, 0]) as [number, number, number];

    const maxDistance = Math.max(...nodes.map(pos =>
      Math.sqrt(
        Math.pow(pos[0] - center[0], 2) +
        Math.pow(pos[1] - center[1], 2) +
        Math.pow(pos[2] - center[2], 2)
      )
    ));

    const margin = 1.5;
    return {
      position: [
        center[0] + maxDistance * margin,
        center[1] + maxDistance * margin,
        center[2] + maxDistance * margin
      ] as [number, number, number],
      target: center
    };
  };

  const positions = simulation
    .filter(n => n && n.reduced_embedding && Array.isArray(n.reduced_embedding) && n.reduced_embedding.length >= 3)
    .map(n => n.reduced_embedding.map((pos: string | number) => {
      let val = typeof pos === 'string' ? parseFloat(pos) : Number(pos);
      return isNaN(val) ? 0 : val * SCENE_SCALE;
    }));

  return (
    <Canvas
      className="w-full h-full"
      frameloop="always"
      gl={{
        antialias: true,
        alpha: false,
        powerPreference: 'high-performance',
        stencil: false,
      }}
      dpr={[1, 2]}
      onCreated={({ gl }) => {
        gl.setClearColor('#050508');
      }}
      onPointerMissed={() => setActive("")}
    >
      <PerspectiveCamera
        makeDefault
        fov={75}
        position={calculateCameraPosition(positions).position}
        near={0.5}
        far={1000}
      />

      {/* Always visible background - outside Suspense */}
      <StarField />

      {/* Camera controls - outside Suspense */}
      <CameraController
        target={cameraTarget}
        onAnimationStart={handleAnimationStart}
        onAnimationEnd={handleAnimationEnd}
      />

      {/* Scene content with loading fallback */}
      <Suspense fallback={<LoadingIndicator />}>
        <Scene
          nodes={simulation}
          activeNode={activeNode}
          setActive={setActive}
          onNavigateNext={onNavigateNext}
          onNavigatePrevious={onNavigatePrevious}
          onToggleTour={onToggleTour}
          onResetToOverview={onResetToOverview}
          onNavigateToIndex={onNavigateToIndex}
          onCameraTargetChange={handleCameraTargetChange}
          screenPositionRef={screenPositionRef}
          onColorClusterInfo={onColorClusterInfo}
        />
      </Suspense>

      {/* Post-processing - with fixes for idle rendering */}
      <PostProcessing />
    </Canvas>
  );
}

export default Render;
