// GLSL functions for color correction
// Injected into MeshPhysicalMaterial via onBeforeCompile

export const colorCorrectionGLSL = `
  vec3 adjustSaturation(vec3 color, float saturation) {
    vec3 luminance = vec3(0.2126, 0.7152, 0.0722);
    float grey = dot(color, luminance);
    return mix(vec3(grey), color, 1.0 + saturation);
  }

  vec3 adjustBrightness(vec3 color, float brightness) {
    return color + brightness;
  }

  vec3 adjustContrast(vec3 color, float contrast) {
    return (color - 0.5) * (1.0 + contrast) + 0.5;
  }

  vec3 applyColorCorrection(vec3 color, float saturation, float brightness, float contrast) {
    color = adjustSaturation(color, saturation);
    color = adjustBrightness(color, brightness);
    color = adjustContrast(color, contrast);
    return clamp(color, 0.0, 1.0);
  }
`;

export const colorCorrectionDefaults = {
  saturation: 0.4,
  brightness: 0.2,
  contrast: 0.3,
};
