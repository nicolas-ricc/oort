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
            {/* Aura effect */}
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

            {/* Planet mesh */}
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


            {/* Keep HTML position relative to the planet */}
            <Html
                position={[0, 1.5, 0]} // Adjust the position as needed
                center
                onClick={onClick}
                className={`text-[10px] select-none transition-colors ease-in-out overflow-x-clip w-[200px]`}

            >
                <div className="relative"
                >
                    {/* Background container with border */}
                    <div className={`bg-black/80 border border-cyan-500/30 p-4 text-cyan-400 uppercase  ${isSelected ? 'text-white' : 'text-lightblue bg-transparent text-transparent' }`}>
                        {/* Description */}
                        {concepts.map((concept) => (
                            <div className="space-y-2 font-mono text-sm" key={concept}>
                                <li className="mt-3 text-xs text-cyan-400 leading-relaxed" onClick={onClick}>
                                    <span >{concept}</span>
                                </li>
                            </div>
                        ))}
                    </div>

                    {/* Decorative angles */}
                    <div className="absolute -top-2 -left-2 w-3 h-3 border-l-4 border-t border-cyan-500"></div>
                    <div className="absolute -top-2 -right-2 w-3 h-3 border-r-4 border-t border-cyan-500"></div>
                    <div className="absolute -bottom-2 -left-2 w-3 h-3 border-l border-b border-cyan-500"></div>
                    <div className="absolute -bottom-2 -right-2 w-3 h-3 border-r border-b border-cyan-500"></div>
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