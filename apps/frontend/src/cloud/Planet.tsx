import { Html, Sparkles, useTexture } from "@react-three/drei";
import { useFrame } from "@react-three/fiber";
import { useRef } from "react";
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
        <group position={position.map(position => parseFloat(position))}>
            {isSelected && (
                <mesh ref={auraRef} position={[0, 0, 0]}>
                    <sphereGeometry args={[1.2 * scale[0], 32, 32]} />
                    <meshStandardMaterial
                        map={texture}
                        roughness={1}
                        emissive={new THREE.Color(0x000000)}
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

                onClick={onClick}
                className="text-2xl select-none transition-colors ease-in-out overflow-x-clip">
                <div>
                    <div className={`bg-black bg-opacity-80 border border-green-700 p-4 ${isSelected ? "opacity-70" : "opacity-20"}`}>
                        {concepts.map((concept) => (
                            <ol className="space-y-2 flex justify-center items-center list-disc pl-2" key={concept}>
                                <li className={`${isSelected ? 'text-white' : 'text-green-400 bg-transparent  opacity-100'} leading-relaxed`} onClick={onClick}>
                                    {concept}
                                </li>
                            </ol>
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