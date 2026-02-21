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
  intensity: number =0 ,
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

// Planet atmosphere palettes — multiple shades per planet from reference palette
// [0] = primary (boosted for glow), [1-2] = additional image swatches
export const planetAtmospherePalettes = {
  sun:     ['#F8A040', '#E86818', '#FDDCB5'],  // warm orange, deep orange, pale peach
  mercury: ['#A8B8C8', '#C6C0BA', '#A09898'],  // cool silver, light gray, medium gray
  earth:   ['#5088C0', '#B0B8C8', '#C0A474'],  // ocean blue, steel blue, sandy tan
  mars:    ['#E4A864', '#A07838', '#B89858'],   // sandy amber, rust brown, khaki
  venus:   ['#F89E70', '#D45820', '#FBCCA8'],   // peach salmon, burnt orange, pale peach
  uranus:  ['#A0DDE8', '#B8CCD4', '#8CA8B4'],   // icy cyan, pale blue-gray, steel blue
  neptune: ['#8878B0', '#7060A0', '#605490'],    // violet, rich purple, deep purple
  saturn:  ['#D4A040', '#C0A870', '#B09060'],    // rich gold, khaki, warm tan
  jupiter: ['#C8A048', '#D4C890', '#B8A870'],    // warm amber, pale gold, khaki gold
  pluto:   ['#7870A8', '#888090', '#B8C0C8'],    // cool violet, gray-purple, pale blue-gray
};

// Interleaved: all primaries, then all shade 2, then all shade 3
// Adjacent cluster indices always get distinct hue families
export const atmosphereColors = [0, 1, 2].flatMap(shade =>
  Object.values(planetAtmospherePalettes).map(p => p[shade])
);

// Get a color based on cluster index
export function getAtmosphereColor(clusterIndex: number): string {
  return atmosphereColors[clusterIndex % atmosphereColors.length];
}
