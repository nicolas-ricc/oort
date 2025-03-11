import { Html, Sparkles, useTexture } from "@react-three/drei";
import { useLoader, useFrame } from "@react-three/fiber";
import { useMemo, useRef, useState } from "react";
import * as THREE from "three";
import { Mesh } from "three";


export function Planet({ texturePath, position, onClick, concepts, isSelected }) {
    const meshRef = useRef(Mesh.prototype);
    const auraRef = useRef(Mesh.prototype);

    const texture = useTexture(texturePath);

    const scale = [1, 32, 32]
    const rotationSpeed = 0.0005

    useFrame((state) => {
        if (meshRef.current) {
            meshRef.current.rotation.y += rotationSpeed;
        }

        if (auraRef.current && isSelected) {
            auraRef.current.scale.x = 1.1 + Math.sin(state.clock.elapsedTime * 2) * 0.05;
            auraRef.current.scale.y = 1.1 + Math.sin(state.clock.elapsedTime * 2) * 0.05;
            auraRef.current.scale.z = 1.1 + Math.sin(state.clock.elapsedTime * 2) * 0.05;
        }
    });


    return (
        <group position={position}>
            {isSelected && (
                <mesh ref={auraRef} position={[0, 0, 0]}>
                    <sphereGeometry args={[1.2 * scale[0], 32, 32]} />
                    <meshBasicMaterial
                        color={0x00ffff}
                        transparent={true}
                        opacity={0.15}
                    />
                </mesh>
            )}

            <mesh
                ref={meshRef}
                onClick={onClick}
            >
                <sphereGeometry args={scale} />
                <meshStandardMaterial
                    map={texture}
                    roughness={0.5}
                    emissive={new THREE.Color(0x000000)}
                />
            </mesh>


            <Html
                position={[0, 1.5, 0]}
                center
                onClick={onClick}
                className={`text-[32px] select-none transition-colors ease-in-out overflow-x-clip inline-block`}>
                <div>
                    <div className={`bg-black/80 border border-green-700 p-4 uppercase ${isSelected ? "opacity-1" : "opacity-30"}`}>
                        {concepts.map((concept) => (
                            <ul className="space-y-2 flex justify-center items-center" key={concept}>
                                <li className={`list-none ${isSelected ? 'text-white' : 'text-green-400 bg-transparent'} leading-relaxed`} onClick={onClick}>
                                    <span >{concept}</span>
                                </li>
                            </ul>
                        ))}
                    </div>
                </div>
            </Html>
            {/* Enhanced sparkles for selected state */}
            <Sparkles
                position={[0, 0, 0]}
                scale={isSelected ? [4, 140, 140] : [3, 128, 128]}
                size={isSelected ? 4 : 3}
                speed={isSelected ? 1 : 0.5}
                opacity={isSelected ? 0.8 : 0.5}
            />
        </group>
    );
}