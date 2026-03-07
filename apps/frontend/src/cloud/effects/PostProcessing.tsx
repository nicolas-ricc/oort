import { useRef, useState, useEffect, Component, ReactNode } from 'react';
import { useFrame } from '@react-three/fiber';
import {
  EffectComposer,
  Bloom,
  Vignette,
  ChromaticAberration,
  Noise,
} from '@react-three/postprocessing';
import { BlendFunction } from 'postprocessing';
import * as THREE from 'three';

type Props = {
  shipSpeed?: number;
  maxSpeed?: number;
};

// Error boundary to prevent postprocessing crashes from taking down the canvas
class PostProcessingErrorBoundary extends Component<
  { children: ReactNode },
  { hasError: boolean }
> {
  state = { hasError: false };
  static getDerivedStateFromError() {
    return { hasError: true };
  }
  componentDidCatch(error: Error) {
    console.warn('PostProcessing disabled due to error:', error.message);
  }
  render() {
    if (this.state.hasError) return null;
    return this.props.children;
  }
}

function PostProcessingEffects({ shipSpeed = 0, maxSpeed = 240 }: Props) {
  const chromaticRef = useRef<any>(null);

  useFrame(() => {
    if (chromaticRef.current) {
      const t = Math.min(shipSpeed / Math.max(maxSpeed, 1), 1);
      const offsetVal = 0.001 + t * 0.007;
      chromaticRef.current.offset.set(offsetVal, offsetVal);
    }
  });

  return (
    <EffectComposer
      multisampling={0}
      disableNormalPass={true}
      frameBufferType={THREE.HalfFloatType}
    >
      <Bloom
        intensity={0.4}
        luminanceThreshold={0.05}
        luminanceSmoothing={0.9}
        mipmapBlur
      />
      <Vignette offset={0.3} darkness={0.8} />
      <ChromaticAberration
        ref={chromaticRef}
        offset={new THREE.Vector2(0.001, 0.001)}
        radialModulation={false}
        modulationOffset={0}
      />
      <Noise
        blendFunction={BlendFunction.SOFT_LIGHT}
        premultiply={true}
        opacity={0.15}
      />
    </EffectComposer>
  );
}

export function PostProcessing(props: Props) {
  // Defer mount by one frame to avoid crashing during initial canvas setup
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(id);
  }, []);

  if (!mounted) return null;

  return (
    <PostProcessingErrorBoundary>
      <PostProcessingEffects {...props} />
    </PostProcessingErrorBoundary>
  );
}
