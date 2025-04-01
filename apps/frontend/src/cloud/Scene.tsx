import { Stars } from "@react-three/drei";
import { useMemo, useState } from "react";
import { Fragment } from "react/jsx-runtime";
import { Planet } from "./Planet";
import aerialView from "./SatelliteNadirView.jpg"

export function Scene({ nodes, activeNode }) {
    const [focusedPlanet, setFocusedPlanet] = useState<number | null>(activeNode);
    useMemo(() => {
        setFocusedPlanet(activeNode)
    }, [activeNode])


    return (
        <>
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