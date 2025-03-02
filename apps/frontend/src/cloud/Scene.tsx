import { OrbitControls, Stars } from "@react-three/drei";
import { useThree } from "@react-three/fiber";
import { useRef, useMemo, useState } from "react";
import { Fragment } from "react/jsx-runtime";
import { Planet } from "./Planet";
import * as THREE from "three";
import aerialView from"./SatelliteNadirView.jpg"

export function Scene({ nodes, activeNode }) {
    const [focusedPlanet, setFocusedPlanet] = useState<number | null>(activeNode);
    const { camera } = useThree();
    const orbitControlsRef = useRef(OrbitControls.prototype);
    useMemo(() => { 
        setFocusedPlanet(activeNode) 
    }, [activeNode])

    useMemo(() => {
        if (focusedPlanet !== null && nodes[focusedPlanet]) {
            console.log("focusedPlanet", focusedPlanet, nodes[focusedPlanet]);
            const targetPos = new THREE.Vector3(...nodes[focusedPlanet][0]);
            const distanceFromCenter = targetPos.length();
            const CAMERA_DISTANCE = Math.max(10, distanceFromCenter * 0.5);
            const CAMERA_HEIGHT = Math.max(4, distanceFromCenter * 0.2);

            const offsetDirection = targetPos.clone().normalize();

            const cameraPosition = targetPos.clone().add(
                offsetDirection.multiplyScalar(CAMERA_DISTANCE)
            );
            cameraPosition.y += CAMERA_HEIGHT;
            camera.position.lerp(cameraPosition, 0.5);
            camera.lookAt(targetPos);
            camera.updateProjectionMatrix();

            if (orbitControlsRef.current) {
                orbitControlsRef.current.target.copy(targetPos);

                orbitControlsRef.current.minDistance = CAMERA_DISTANCE * 2;
                orbitControlsRef.current.maxDistance = CAMERA_DISTANCE * 4;

                orbitControlsRef.current.update();
            }
        } else {
            const resetPosition = new THREE.Vector3(30, 30, 30);
            camera.position.lerp(resetPosition, 0.5);
            camera.lookAt(0, 0, 0);
            camera.updateProjectionMatrix();

            if (orbitControlsRef.current) {
                orbitControlsRef.current.target.set(0, 0, 0);
                orbitControlsRef.current.minDistance = 10;
                orbitControlsRef.current.maxDistance = 100;
                orbitControlsRef.current.update();
            }
        }
    }, [focusedPlanet, nodes, camera]);

    return (
        <>
            <OrbitControls
                ref={orbitControlsRef}
                enabled={true}
                enableZoom={true}
                enablePan={true}
                enableRotate={true}
                dampingFactor={0.1}
                rotateSpeed={0.5} />
                        <color attach="background" args={['#060605']} />
            <ambientLight intensity={0.5} />
            <pointLight position={[0, 0, 2]} intensity={5} />
            <Stars radius={200} depth={50} count={3000} factor={7} />

            {/* Planets with Textures */}
            {nodes.map((node, idx) => (
                <Fragment key={idx}>
                    <Planet
                        texturePath={aerialView}
                        position={node[0]}
                        concepts={node[1]}
                        onClick={() => setFocusedPlanet(idx)}
                        isSelected={focusedPlanet === idx}

                    />

                </Fragment>
            ))}

        </>
    );
}