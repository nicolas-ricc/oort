import * as THREE from 'three';

// Fresnel-based atmosphere shader for planet glow effect
export const atmosphereVertexShader = `
  varying vec3 vNormal;
  varying vec3 vPosition;

  void main() {
    vNormal = normalize(normalMatrix * normal);
    vPosition = (modelViewMatrix * vec4(position, 1.0)).xyz;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

export const atmosphereFragmentShader = `
  uniform vec3 glowColor;
  uniform float intensity;
  uniform float power;

  varying vec3 vNormal;
  varying vec3 vPosition;

  void main() {
    // Calculate view direction
    vec3 viewDirection = normalize(-vPosition);

    // Fresnel effect - stronger glow at edges
    float fresnel = 1.0 - dot(viewDirection, vNormal);
    fresnel = pow(fresnel, power);

    // Apply intensity
    float alpha = fresnel * intensity;

    gl_FragColor = vec4(glowColor, alpha);
  }
`;

export function createAtmosphereMaterial(
  color: THREE.Color | string = '#4da6ff',
  intensity: number = 0.6,
  power: number = 2.0
): THREE.ShaderMaterial {
  return new THREE.ShaderMaterial({
    vertexShader: atmosphereVertexShader,
    fragmentShader: atmosphereFragmentShader,
    uniforms: {
      glowColor: { value: new THREE.Color(color) },
      intensity: { value: intensity },
      power: { value: power },
    },
    side: THREE.BackSide,
    transparent: true,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
  });
}

// Pre-defined color palettes for different planet types
export const atmosphereColors = {
  blue: '#2196ff',
  green: '#00ff7f',
  purple: '#b84dff',
  orange: '#ff6b2b',
  cyan: '#00e5ff',
  pink: '#ff4d94',
  gold: '#ffc400',
  red: '#ff3333',
};

// Get a color based on cluster index
export function getAtmosphereColor(clusterIndex: number): string {
  const colors = Object.values(atmosphereColors);
  return colors[clusterIndex % colors.length];
}
