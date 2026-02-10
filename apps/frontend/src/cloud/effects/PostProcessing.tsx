import {
  EffectComposer,
  Bloom,
  Vignette,
  ChromaticAberration,
} from '@react-three/postprocessing';
import * as THREE from 'three';

export function PostProcessing() {
  return (
    <EffectComposer
      multisampling={0}        // Disable MSAA - can cause render issues
      disableNormalPass={true} // Skip normal pass - not needed for these effects
      frameBufferType={THREE.HalfFloatType} // Better color precision
    >
      {/* Global effects only - color correction moved to planet material shaders */}
      <Bloom
        intensity={0.4}
        luminanceThreshold={0.05}
        luminanceSmoothing={0.9}
        mipmapBlur
      />
      <Vignette offset={0.3} darkness={0.8} />
      <ChromaticAberration
        offset={new THREE.Vector2(0.001, 0.001)}
        radialModulation={false}
        modulationOffset={0}
      />
    </EffectComposer>
  );
}
