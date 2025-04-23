import { Stars } from "@react-three/drei";
import { Fragment } from "react/jsx-runtime";
import { Planet } from "./Planet";
import { textures } from "@/assets/textures";
import { useThree } from "@react-three/fiber";
import * as THREE from "three";

export function Scene({ nodes, activeNode, setActive }) {
    useThree(({ camera }) => {
        camera.rotation.set(THREE.MathUtils.degToRad(30), 0, 0);
      });

    return (
        <>
            <ambientLight intensity={0.5} />
            <pointLight position={[0, 0, 2]} intensity={5} />
            <Stars radius={200} depth={50} count={3000} factor={7} />

            {/* Planets with Textures */}
            {nodes.map((node, idx) => {
                console.log(node)
                console.log(textures)
                return <Fragment key={node.reduced_embedding.join("")}>
                    <Planet
                        texturePath={textures[0]}
                        position={node.reduced_embedding}
                        concepts={node.concepts}
                        onClick={() => setActive(node.reduced_embedding.join(""))}
                        isSelected={activeNode === node.reduced_embedding.join("")}
                    />

                </Fragment>
            }
            )}
        </>
    );
}