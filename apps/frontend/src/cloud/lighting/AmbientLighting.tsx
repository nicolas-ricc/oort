import { SCENE_SCALE } from "../hooks/useSceneScale";

export function AmbientLighting() {
  return (
    <>
      <ambientLight intensity={0.5} />
      <pointLight
        position={[0, 0, 2 * SCENE_SCALE]}
        intensity={5}
        distance={50 * SCENE_SCALE}
      />
    </>
  );
}
