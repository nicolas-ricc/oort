import { useMemo } from 'react';
import { MeshPhysicalMaterial, Texture } from 'three';
import { colorCorrectionGLSL, colorCorrectionDefaults } from '../shaders/ColorCorrectionShader';

type ColorCorrectionOptions = {
  saturation: number;
  brightness: number;
  contrast: number;
};

export function useColorCorrectedMaterial(
  texture: Texture,
  options: ColorCorrectionOptions = colorCorrectionDefaults
) {
  return useMemo(() => {
    const material = new MeshPhysicalMaterial({
      map: texture,
      roughness: 0.4,
      metalness: 0.1,
      clearcoat: 0.3,
      clearcoatRoughness: 0.4,
    });

    // Set onBeforeCompile BEFORE shader compilation
    material.onBeforeCompile = (shader) => {
      shader.uniforms.uSaturation = { value: options.saturation };
      shader.uniforms.uBrightness = { value: options.brightness };
      shader.uniforms.uContrast = { value: options.contrast };

      shader.fragmentShader = `
        uniform float uSaturation;
        uniform float uBrightness;
        uniform float uContrast;
        ${colorCorrectionGLSL}
        ${shader.fragmentShader}
      `;

      shader.fragmentShader = shader.fragmentShader.replace(
        '#include <output_fragment>',
        `
        #include <output_fragment>
        gl_FragColor.rgb = applyColorCorrection(gl_FragColor.rgb, uSaturation, uBrightness, uContrast);
        `
      );
    };

    // Force unique shader program - critical for onBeforeCompile to work
    material.customProgramCacheKey = () =>
      `color-corrected-${options.saturation}-${options.brightness}-${options.contrast}`;

    return material;
  }, [texture, options.saturation, options.brightness, options.contrast]);
}
