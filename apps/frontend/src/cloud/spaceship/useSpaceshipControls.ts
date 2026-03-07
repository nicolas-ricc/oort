import { useRef, useEffect, useCallback, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { SCENE_SCALE } from '../hooks/useSceneScale';

export type SpaceshipState = {
  position: THREE.Vector3;
  velocity: THREE.Vector3;
  quaternion: THREE.Quaternion;
  speed: number;
  isBoosting: boolean;
};

type SpaceshipConfig = {
  thrust: number;
  strafe: number;
  boostMultiplier: number;
  damping: number;
  maxSpeed: number;
  mouseSensitivity: number;
  rollSpeed: number;
};

const DEFAULT_CONFIG: SpaceshipConfig = {
  thrust: 20 * SCENE_SCALE,
  strafe: 15 * SCENE_SCALE,
  boostMultiplier: 3.0,
  damping: 0.96,
  maxSpeed: 40 * SCENE_SCALE,
  mouseSensitivity: 0.002,
  rollSpeed: 2.0,
};

function easeInOutQuad(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
}

type FlyToTarget = {
  position: THREE.Vector3;
  lookAt: THREE.Vector3;
  startPosition: THREE.Vector3;
  startQuaternion: THREE.Quaternion;
  targetQuaternion: THREE.Quaternion;
  duration: number;
  startTime: number | null;
};

export function useSpaceshipControls(
  controlsEnabled: boolean,
  config?: Partial<SpaceshipConfig>
) {
  const { camera, gl } = useThree();
  const cfg = { ...DEFAULT_CONFIG, ...config };

  const stateRef = useRef<SpaceshipState>({
    position: camera.position.clone(),
    velocity: new THREE.Vector3(),
    quaternion: camera.quaternion.clone(),
    speed: 0,
    isBoosting: false,
  });

  const keysRef = useRef(new Set<string>());
  const flyToRef = useRef<FlyToTarget | null>(null);
  const [isAutoFlying, setIsAutoFlying] = useState(false);
  const pointerLockedRef = useRef(false);

  // Mouse delta accumulator (read and reset each frame)
  const mouseDeltaRef = useRef({ x: 0, y: 0 });

  // Track pointer lock state
  useEffect(() => {
    const onLockChange = () => {
      pointerLockedRef.current = document.pointerLockElement === gl.domElement;
    };
    document.addEventListener('pointerlockchange', onLockChange);
    return () => document.removeEventListener('pointerlockchange', onLockChange);
  }, [gl.domElement]);

  // Key listeners
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Don't capture keys when typing in inputs
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      keysRef.current.add(e.key.toLowerCase());
    };
    const onKeyUp = (e: KeyboardEvent) => {
      keysRef.current.delete(e.key.toLowerCase());
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
    };
  }, []);

  // Mouse move listener (pointer lock mode only)
  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!pointerLockedRef.current) return;
      mouseDeltaRef.current.x += e.movementX;
      mouseDeltaRef.current.y += e.movementY;
    };
    document.addEventListener('mousemove', onMouseMove);
    return () => document.removeEventListener('mousemove', onMouseMove);
  }, []);

  // Click to engage pointer lock
  useEffect(() => {
    const onClick = () => {
      if (controlsEnabled && !pointerLockedRef.current) {
        gl.domElement.requestPointerLock();
      }
    };
    gl.domElement.addEventListener('click', onClick);
    return () => gl.domElement.removeEventListener('click', onClick);
  }, [gl.domElement, controlsEnabled]);

  // Release pointer lock when controls disabled
  useEffect(() => {
    if (!controlsEnabled && pointerLockedRef.current) {
      document.exitPointerLock();
    }
  }, [controlsEnabled]);

  const flyTo = useCallback((target: THREE.Vector3, lookAt: THREE.Vector3, duration = 1.4) => {
    const state = stateRef.current;
    // Kill velocity for smooth transition
    state.velocity.set(0, 0, 0);

    // Compute target quaternion from lookAt
    const tempCamera = new THREE.Object3D();
    tempCamera.position.copy(target);
    tempCamera.lookAt(lookAt);

    flyToRef.current = {
      position: target.clone(),
      lookAt: lookAt.clone(),
      startPosition: state.position.clone(),
      startQuaternion: state.quaternion.clone(),
      targetQuaternion: tempCamera.quaternion.clone(),
      duration,
      startTime: null,
    };
    setIsAutoFlying(true);
  }, []);

  useFrame((frameState, delta) => {
    const state = stateRef.current;

    // Handle auto-flight
    if (flyToRef.current) {
      const ft = flyToRef.current;
      if (ft.startTime === null) {
        ft.startTime = frameState.clock.elapsedTime;
      }

      const elapsed = frameState.clock.elapsedTime - ft.startTime;
      const progress = Math.min(elapsed / ft.duration, 1);
      const eased = easeInOutQuad(progress);

      state.position.lerpVectors(ft.startPosition, ft.position, eased);
      state.quaternion.slerpQuaternions(ft.startQuaternion, ft.targetQuaternion, eased);

      camera.position.copy(state.position);
      camera.quaternion.copy(state.quaternion);

      if (progress >= 1) {
        flyToRef.current = null;
        setIsAutoFlying(false);
        state.speed = 0;
      }
      return;
    }

    if (!controlsEnabled) {
      // Still sync camera to state (so nothing jumps when re-enabled)
      camera.position.copy(state.position);
      camera.quaternion.copy(state.quaternion);
      state.speed = state.velocity.length();
      return;
    }

    const keys = keysRef.current;

    // Mouse rotation (pitch/yaw) — only when pointer locked
    const dx = mouseDeltaRef.current.x;
    const dy = mouseDeltaRef.current.y;
    mouseDeltaRef.current.x = 0;
    mouseDeltaRef.current.y = 0;

    if (pointerLockedRef.current && (dx !== 0 || dy !== 0)) {
      const euler = new THREE.Euler(0, 0, 0, 'YXZ');
      euler.setFromQuaternion(state.quaternion, 'YXZ');
      euler.y -= dx * cfg.mouseSensitivity;
      euler.x -= dy * cfg.mouseSensitivity;
      euler.x = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, euler.x));
      state.quaternion.setFromEuler(euler);
    }

    // Roll (Q/E)
    if (keys.has('q')) {
      const rollQuat = new THREE.Quaternion();
      const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(state.quaternion);
      rollQuat.setFromAxisAngle(forward, cfg.rollSpeed * delta);
      state.quaternion.premultiply(rollQuat);
    }
    if (keys.has('e')) {
      const rollQuat = new THREE.Quaternion();
      const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(state.quaternion);
      rollQuat.setFromAxisAngle(forward, -cfg.rollSpeed * delta);
      state.quaternion.premultiply(rollQuat);
    }

    // Thrust direction vectors (in camera space)
    const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(state.quaternion);
    const right = new THREE.Vector3(1, 0, 0).applyQuaternion(state.quaternion);
    const up = new THREE.Vector3(0, 1, 0);

    const isBoosting = keys.has('shift');
    const boost = isBoosting ? cfg.boostMultiplier : 1.0;
    state.isBoosting = isBoosting;

    // WASD thrust
    if (keys.has('w')) state.velocity.addScaledVector(forward, cfg.thrust * boost * delta);
    if (keys.has('s')) state.velocity.addScaledVector(forward, -cfg.thrust * boost * delta);
    if (keys.has('a')) state.velocity.addScaledVector(right, -cfg.strafe * boost * delta);
    if (keys.has('d')) state.velocity.addScaledVector(right, cfg.strafe * boost * delta);

    // Vertical (Space/Control)
    if (keys.has(' ')) state.velocity.addScaledVector(up, cfg.strafe * boost * delta);
    if (keys.has('control')) state.velocity.addScaledVector(up, -cfg.strafe * boost * delta);

    // Damping
    state.velocity.multiplyScalar(cfg.damping);

    // Clamp speed
    const maxSpd = cfg.maxSpeed * (isBoosting ? cfg.boostMultiplier : 1.0);
    if (state.velocity.length() > maxSpd) {
      state.velocity.setLength(maxSpd);
    }

    // Integrate position
    state.position.addScaledVector(state.velocity, delta);
    state.speed = state.velocity.length();

    // Update camera
    camera.position.copy(state.position);
    camera.quaternion.copy(state.quaternion);
  });

  return {
    stateRef,
    flyTo,
    isAutoFlying,
    maxSpeed: cfg.maxSpeed * cfg.boostMultiplier,
  };
}
