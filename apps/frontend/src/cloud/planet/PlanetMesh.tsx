import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

type Props = {
  radius: number;
  clusterIndex: number;
  atmosphereColor: string;
  isSelected: boolean;
  onClick: () => void;
  seed: number;
};

// --- Planet palettes (8, matching atmosphere color indices) ---

type PlanetPalette = {
  colors: [string, string, string, string, string, string, string];
  thresholds: [number, number, number, number, number, number];
  specularBand: number;
  displacementScale: number;
  noiseScale: number;
  octaves: number;
};

const PALETTES: PlanetPalette[] = [
  // 0 - Ocean: deep/shallow blue seas, sandy shores, green lowlands, rocky highlands, snow
  {
    colors: ['#05124d', '#0d2e61', '#9e8a64', '#385a28', '#594d38', '#73675a', '#e6eaf0'],
    thresholds: [-0.3, -0.05, 0.0, 0.1, 0.25, 0.4],
    specularBand: 0.0,
    displacementScale: 0.08,
    noiseScale: 2.5,
    octaves: 6,
  },
  // 1 - Verdant: dense jungle greens, mossy depths, pale highlands
  {
    colors: ['#0a2e12', '#1a5c28', '#2d8c3e', '#5ab84e', '#8cd470', '#c4e8a0', '#f0f5e0'],
    thresholds: [-0.2, -0.05, 0.05, 0.15, 0.3, 0.45],
    specularBand: -99.0,
    displacementScale: 0.07,
    noiseScale: 3.0,
    octaves: 6,
  },
  // 2 - Mystic: purple-to-lavender crystalline alien terrain
  {
    colors: ['#1a0a33', '#3d1a6e', '#6b30a8', '#9455d4', '#b880e8', '#d4aaf5', '#f0e0ff'],
    thresholds: [-0.25, -0.05, 0.05, 0.15, 0.3, 0.45],
    specularBand: -0.05,
    displacementScale: 0.06,
    noiseScale: 2.8,
    octaves: 6,
  },
  // 3 - Volcanic: bright lava rivers in lowlands, dark rock peaks
  {
    colors: ['#ff4400', '#ff6a00', '#cc3300', '#3d1a0a', '#2b1008', '#1a0a05', '#4d3020'],
    thresholds: [-0.2, -0.05, 0.0, 0.1, 0.25, 0.4],
    specularBand: -0.05,
    displacementScale: 0.12,
    noiseScale: 2.2,
    octaves: 7,
  },
  // 4 - Ice: frozen blue-white, broad specular, low displacement
  {
    colors: ['#1a3a5c', '#2e5a80', '#5a8ab0', '#8abade', '#b0d4f0', '#d8ecf8', '#f5faff'],
    thresholds: [-0.25, -0.1, 0.0, 0.1, 0.2, 0.35],
    specularBand: 0.1,
    displacementScale: 0.03,
    noiseScale: 2.0,
    octaves: 5,
  },
  // 5 - Coral: pink/magenta oceans, warm sandy transitions
  {
    colors: ['#6e1a3a', '#a83060', '#d45a80', '#e8a090', '#f0c8a0', '#f5e0c8', '#fff0e8'],
    thresholds: [-0.25, -0.05, 0.05, 0.15, 0.3, 0.45],
    specularBand: -0.05,
    displacementScale: 0.06,
    noiseScale: 2.6,
    octaves: 6,
  },
  // 6 - Desert: sandy amber, minimal displacement
  {
    colors: ['#6e5020', '#8c6830', '#b08848', '#cca860', '#e0c880', '#f0e0a8', '#f8f0d8'],
    thresholds: [-0.2, -0.05, 0.05, 0.15, 0.3, 0.45],
    specularBand: -99.0,
    displacementScale: 0.04,
    noiseScale: 2.0,
    octaves: 4,
  },
  // 7 - Inferno: deep crimson to orange-hot
  {
    colors: ['#1a0505', '#4d0a0a', '#801515', '#b03020', '#d45030', '#f08040', '#ffc060'],
    thresholds: [-0.25, -0.1, 0.0, 0.1, 0.25, 0.4],
    specularBand: -99.0,
    displacementScale: 0.1,
    noiseScale: 2.4,
    octaves: 7,
  },
];

// --- GLSL Shaders ---

const vertexShader = /* glsl */ `
  //
  // 3D Simplex Noise (Stefan Gustavson)
  //
  vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
  vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
  vec4 permute(vec4 x) { return mod289(((x * 34.0) + 1.0) * x); }
  vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }

  float snoise(vec3 v) {
    const vec2 C = vec2(1.0 / 6.0, 1.0 / 3.0);
    const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);

    vec3 i = floor(v + dot(v, C.yyy));
    vec3 x0 = v - i + dot(i, C.xxx);

    vec3 g = step(x0.yzx, x0.xyz);
    vec3 l = 1.0 - g;
    vec3 i1 = min(g.xyz, l.zxy);
    vec3 i2 = max(g.xyz, l.zxy);

    vec3 x1 = x0 - i1 + C.xxx;
    vec3 x2 = x0 - i2 + C.yyy;
    vec3 x3 = x0 - D.yyy;

    i = mod289(i);
    vec4 p = permute(
      permute(
        permute(i.z + vec4(0.0, i1.z, i2.z, 1.0))
        + i.y + vec4(0.0, i1.y, i2.y, 1.0))
      + i.x + vec4(0.0, i1.x, i2.x, 1.0));

    float n_ = 0.142857142857;
    vec3 ns = n_ * D.wyz - D.xzx;

    vec4 j = p - 49.0 * floor(p * ns.z * ns.z);
    vec4 x_ = floor(j * ns.z);
    vec4 y_ = floor(j - 7.0 * x_);

    vec4 x = x_ * ns.x + ns.yyyy;
    vec4 y = y_ * ns.x + ns.yyyy;
    vec4 h = 1.0 - abs(x) - abs(y);

    vec4 b0 = vec4(x.xy, y.xy);
    vec4 b1 = vec4(x.zw, y.zw);

    vec4 s0 = floor(b0) * 2.0 + 1.0;
    vec4 s1 = floor(b1) * 2.0 + 1.0;
    vec4 sh = -step(h, vec4(0.0));

    vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;

    vec3 p0 = vec3(a0.xy, h.x);
    vec3 p1 = vec3(a0.zw, h.y);
    vec3 p2 = vec3(a1.xy, h.z);
    vec3 p3 = vec3(a1.zw, h.w);

    vec4 norm = taylorInvSqrt(vec4(dot(p0,p0), dot(p1,p1), dot(p2,p2), dot(p3,p3)));
    p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;

    vec4 m = max(0.6 - vec4(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), 0.0);
    m = m * m;
    return 42.0 * dot(m * m, vec4(dot(p0,x0), dot(p1,x1), dot(p2,x2), dot(p3,x3)));
  }

  // Fractional Brownian Motion for layered detail
  float fbm(vec3 p, int octaves) {
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 1.0;
    for (int i = 0; i < 8; i++) {
      if (i >= octaves) break;
      value += amplitude * snoise(p * frequency);
      frequency *= 2.0;
      amplitude *= 0.5;
    }
    return value;
  }

  uniform float uTime;
  uniform float uDisplacementScale;
  uniform float uNoiseScale;
  uniform int uOctaves;
  uniform vec3 uSeedOffset;

  varying float vElevation;
  varying vec3 vNormal;
  varying vec3 vWorldPosition;
  varying vec3 vViewDir;

  void main() {
    vec3 pos = position;
    vec3 dir = normalize(pos);

    // Sample noise at the surface point, offset by seed for unique terrain
    vec3 samplePos = dir * uNoiseScale + uSeedOffset + vec3(0.0, 0.0, uTime * 0.01);
    float elevation = fbm(samplePos, uOctaves);

    // Clamp and remap: below a threshold is ocean (flat), above is terrain
    float terrainElevation = max(elevation, -0.05);

    // Displace vertex along normal
    pos += dir * terrainElevation * uDisplacementScale;

    vElevation = elevation;
    vWorldPosition = (modelMatrix * vec4(pos, 1.0)).xyz;
    vViewDir = normalize(cameraPosition - vWorldPosition);

    // Approximate displaced normal via central differences
    float eps = 0.01;
    vec3 tangent1 = normalize(cross(dir, vec3(0.0, 1.0, 0.0)));
    if (length(cross(dir, vec3(0.0, 1.0, 0.0))) < 0.001) {
      tangent1 = normalize(cross(dir, vec3(1.0, 0.0, 0.0)));
    }
    vec3 tangent2 = normalize(cross(dir, tangent1));

    vec3 neighbor1 = normalize(dir + tangent1 * eps);
    vec3 neighbor2 = normalize(dir + tangent2 * eps);
    float e1 = fbm(neighbor1 * uNoiseScale + uSeedOffset + vec3(0.0, 0.0, uTime * 0.01), uOctaves);
    float e2 = fbm(neighbor2 * uNoiseScale + uSeedOffset + vec3(0.0, 0.0, uTime * 0.01), uOctaves);
    e1 = max(e1, -0.05);
    e2 = max(e2, -0.05);

    vec3 p1 = neighbor1 * (1.0 + e1 * uDisplacementScale);
    vec3 p2 = neighbor2 * (1.0 + e2 * uDisplacementScale);
    vec3 p0 = dir * (1.0 + terrainElevation * uDisplacementScale);

    vNormal = normalize(cross(p1 - p0, p2 - p0));
    vNormal = normalize(normalMatrix * vNormal);

    gl_Position = projectionMatrix * modelViewMatrix * vec4(pos, 1.0);
  }
`;

const fragmentShader = /* glsl */ `
  uniform vec3 uLightDir;
  uniform float uTime;

  uniform vec3 uColor0;
  uniform vec3 uColor1;
  uniform vec3 uColor2;
  uniform vec3 uColor3;
  uniform vec3 uColor4;
  uniform vec3 uColor5;
  uniform vec3 uColor6;

  uniform float uThreshold0;
  uniform float uThreshold1;
  uniform float uThreshold2;
  uniform float uThreshold3;
  uniform float uThreshold4;
  uniform float uThreshold5;

  uniform float uSpecularBand;
  uniform vec3 uEmissiveColor;

  varying float vElevation;
  varying vec3 vNormal;
  varying vec3 vWorldPosition;
  varying vec3 vViewDir;

  void main() {
    float e = vElevation;

    // Elevation-based color bands
    vec3 color;
    if (e < uThreshold0) {
      color = uColor0;
    } else if (e < uThreshold1) {
      color = mix(uColor0, uColor1, smoothstep(uThreshold0, uThreshold1, e));
    } else if (e < uThreshold2) {
      color = mix(uColor1, uColor2, smoothstep(uThreshold1, uThreshold2, e));
    } else if (e < uThreshold3) {
      color = mix(uColor2, uColor3, smoothstep(uThreshold2, uThreshold3, e));
    } else if (e < uThreshold4) {
      color = mix(uColor3, uColor4, smoothstep(uThreshold3, uThreshold4, e));
    } else if (e < uThreshold5) {
      color = mix(uColor4, uColor5, smoothstep(uThreshold4, uThreshold5, e));
    } else {
      color = mix(uColor5, uColor6, smoothstep(uThreshold5, uThreshold5 + 0.15, e));
    }

    // Diffuse lighting
    vec3 lightDir = normalize(uLightDir);
    float NdotL = max(dot(vNormal, lightDir), 0.0);
    float diffuse = NdotL * 0.85 + 0.15;

    // Specular highlight for low-elevation bands (ocean-like)
    float specular = 0.0;
    if (e < uSpecularBand) {
      vec3 halfDir = normalize(lightDir + vViewDir);
      specular = pow(max(dot(vNormal, halfDir), 0.0), 64.0) * 0.5;
    }

    // Elevation-based emissive glow: bright at low elevations, dim at peaks
    float t = clamp((e - uThreshold0) / (uThreshold5 - uThreshold0), 0.0, 1.0);
    float emissiveStrength = mix(0.20, 0.01, t);
    vec3 emissive = uEmissiveColor * emissiveStrength;

    vec3 finalColor = color * diffuse + vec3(specular) + emissive;

    gl_FragColor = vec4(finalColor, 1.0);
  }
`;

function hexToVec3(hex: string): THREE.Vector3 {
  const c = new THREE.Color(hex);
  return new THREE.Vector3(c.r, c.g, c.b);
}

function seedToOffset(seed: number): THREE.Vector3 {
  // Spread seeds across noise space so planets look distinct
  const x = Math.sin(seed * 12.9898) * 43758.5453 % 100;
  const y = Math.sin(seed * 78.233) * 43758.5453 % 100;
  const z = Math.sin(seed * 45.164) * 43758.5453 % 100;
  return new THREE.Vector3(x, y, z);
}

export function PlanetMesh({ radius, clusterIndex, atmosphereColor, isSelected, onClick, seed }: Props) {
  const meshRef = useRef<THREE.Mesh>(null);
  const palette = PALETTES[clusterIndex % PALETTES.length];
  const emissiveColor = new THREE.Color(atmosphereColor);

  const uniforms = useMemo(() => ({
    uTime: { value: 0 },
    uDisplacementScale: { value: palette.displacementScale },
    uNoiseScale: { value: palette.noiseScale },
    uOctaves: { value: palette.octaves },
    uSeedOffset: { value: seedToOffset(seed) },
    uLightDir: { value: new THREE.Vector3(5, 3, 5).normalize() },

    uColor0: { value: hexToVec3(palette.colors[0]) },
    uColor1: { value: hexToVec3(palette.colors[1]) },
    uColor2: { value: hexToVec3(palette.colors[2]) },
    uColor3: { value: hexToVec3(palette.colors[3]) },
    uColor4: { value: hexToVec3(palette.colors[4]) },
    uColor5: { value: hexToVec3(palette.colors[5]) },
    uColor6: { value: hexToVec3(palette.colors[6]) },

    uThreshold0: { value: palette.thresholds[0] },
    uThreshold1: { value: palette.thresholds[1] },
    uThreshold2: { value: palette.thresholds[2] },
    uThreshold3: { value: palette.thresholds[3] },
    uThreshold4: { value: palette.thresholds[4] },
    uThreshold5: { value: palette.thresholds[5] },

    uSpecularBand: { value: palette.specularBand },
    uEmissiveColor: { value: new THREE.Vector3(emissiveColor.r, emissiveColor.g, emissiveColor.b) },
  }), [palette, seed, emissiveColor]);

  useFrame((_state, delta) => {
    if (meshRef.current) {
      meshRef.current.rotation.y += delta * 0.05;
      (meshRef.current.material as THREE.ShaderMaterial).uniforms.uTime.value += delta;
    }
  });

  return (
    <mesh ref={meshRef} onClick={onClick} scale={radius}>
      <sphereGeometry args={[1, 64, 64]} />
      <shaderMaterial
        vertexShader={vertexShader}
        fragmentShader={fragmentShader}
        uniforms={uniforms}
      />
    </mesh>
  );
}
