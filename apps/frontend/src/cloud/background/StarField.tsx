import { Stars } from "@react-three/drei";
import { SCENE_SCALE } from "../hooks/useSceneScale";

export function StarField() {
  return (
    <>
      {/* Far layer - slowest */}
      <Stars
        radius={300 * SCENE_SCALE}
        depth={100 * SCENE_SCALE}
        count={5000}
        factor={8}
        saturation={0.1}
        fade
        speed={0.5}
      />
      {/* Mid layer */}
      <Stars
        radius={100 * SCENE_SCALE}
        depth={30 * SCENE_SCALE}
        count={2000}
        factor={4}
        saturation={0.3}
        fade
        speed={1}
      />
      {/* Near layer - fastest */}
      <Stars
        radius={50 * SCENE_SCALE}
        depth={20 * SCENE_SCALE}
        count={500}
        factor={2}
        saturation={0}
        fade
        speed={0.2}
      />
    </>
  );
}
