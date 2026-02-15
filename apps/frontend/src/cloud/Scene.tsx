import { Fragment, useMemo, useEffect, useCallback, MutableRefObject } from "react";
import { Planet } from "./planet/Planet";
import { textures } from "@/assets/textures";
import { useThree, useFrame } from "@react-three/fiber";
import { AmbientLighting } from "./lighting/AmbientLighting";
import { ClusterLights } from "./lighting/ClusterLights";
import { SCENE_SCALE } from "./hooks/useSceneScale";
import * as THREE from "three";

function safeParseEmbedding(embedding: number[] | string[], scaleFactor = SCENE_SCALE): number[] {
    if (!Array.isArray(embedding)) {
        console.warn("Embedding is not an array:", embedding);
        return [0, 0, 0];
    }

    return embedding.map(p => {
        let val = typeof p === 'string' ? parseFloat(p) : Number(p);
        return isNaN(val) ? 0 : val * scaleFactor;
    });
}

function avoidCollisions(nodes: any[], minDistance = 4) {
    const validNodes = nodes.filter(node => {
        const isValid = node && node.reduced_embedding && Array.isArray(node.reduced_embedding) && node.reduced_embedding.length >= 3;
        if (!isValid) {
            console.warn("Invalid node:", node);
        }
        return isValid;
    });

    const adjustedNodes = validNodes.map(node => ({
        ...node,
        reduced_embedding: [...node.reduced_embedding]
    }));

    // Work in raw (unscaled) coordinates to avoid double-scaling
    const planetRadius = 1;
    const safeDistance = minDistance + planetRadius * 2;
    const maxPasses = 5;

    for (let pass = 0; pass < maxPasses; pass++) {
        let hadCollision = false;

        for (let i = 0; i < adjustedNodes.length; i++) {
            for (let j = i + 1; j < adjustedNodes.length; j++) {
                const pos1 = adjustedNodes[i].reduced_embedding;
                const pos2 = adjustedNodes[j].reduced_embedding;

                const distance = Math.sqrt(
                    Math.pow(pos1[0] - pos2[0], 2) +
                    Math.pow(pos1[1] - pos2[1], 2) +
                    Math.pow(pos1[2] - pos2[2], 2)
                );

                if (distance < safeDistance) {
                    hadCollision = true;
                    const direction = [
                        pos2[0] - pos1[0],
                        pos2[1] - pos1[1],
                        pos2[2] - pos1[2]
                    ];
                    const length = Math.sqrt(direction[0] ** 2 + direction[1] ** 2 + direction[2] ** 2);
                    const normalized = length > 0.0001
                        ? direction.map(d => d / length)
                        : [1, 0, 0]; // fallback for coincident points

                    const pushDistance = (safeDistance - distance) / 2;

                    adjustedNodes[i].reduced_embedding = [
                        pos1[0] - normalized[0] * pushDistance,
                        pos1[1] - normalized[1] * pushDistance,
                        pos1[2] - normalized[2] * pushDistance
                    ].map(val => isNaN(val) ? 0 : val);

                    adjustedNodes[j].reduced_embedding = [
                        pos2[0] + normalized[0] * pushDistance,
                        pos2[1] + normalized[1] * pushDistance,
                        pos2[2] + normalized[2] * pushDistance
                    ].map(val => isNaN(val) ? 0 : val);
                }
            }
        }

        if (!hadCollision) break;
    }

    return adjustedNodes;
}

function calculateDistance(pos1: number[], pos2: number[]): number {
    return Math.sqrt(
        Math.pow(pos1[0] - pos2[0], 2) +
        Math.pow(pos1[1] - pos2[1], 2) +
        Math.pow(pos1[2] - pos2[2], 2)
    );
}

function getTextureIndexForPlanet(position: number[], textureCount: number): number {
    const hash = position[0] * 500 + position[1] * 50 + position[2] * 5;
    return Math.abs(Math.floor(hash)) % textureCount;
}

// DBSCAN clustering algorithm
function dbscan3D(points: { position: number[]; node: any }[], epsilon = 8, minPoints = 2) {
    const clusters: number[][] = [];
    const visited = new Set<number>();
    const noise = new Set<number>();

    function getNeighbors(pointIndex: number): number[] {
        const neighbors: number[] = [];
        const currentPoint = points[pointIndex];

        for (let i = 0; i < points.length; i++) {
            if (i !== pointIndex) {
                const distance = calculateDistance(currentPoint.position, points[i].position);
                if (distance <= epsilon) {
                    neighbors.push(i);
                }
            }
        }
        return neighbors;
    }

    function expandCluster(pointIndex: number, neighbors: number[], clusterId: number) {
        clusters[clusterId].push(pointIndex);

        for (let i = 0; i < neighbors.length; i++) {
            const neighborIndex = neighbors[i];

            if (!visited.has(neighborIndex)) {
                visited.add(neighborIndex);
                const neighborNeighbors = getNeighbors(neighborIndex);

                if (neighborNeighbors.length >= minPoints) {
                    neighbors.push(...neighborNeighbors.filter(n => !neighbors.includes(n)));
                }
            }

            let inCluster = false;
            for (let j = 0; j < clusters.length; j++) {
                if (clusters[j].includes(neighborIndex)) {
                    inCluster = true;
                    break;
                }
            }
            if (!inCluster) {
                clusters[clusterId].push(neighborIndex);
            }
        }
    }

    for (let i = 0; i < points.length; i++) {
        if (visited.has(i)) continue;

        visited.add(i);
        const neighbors = getNeighbors(i);

        if (neighbors.length < minPoints) {
            noise.add(i);
        } else {
            const clusterId = clusters.length;
            clusters.push([]);
            expandCluster(i, neighbors, clusterId);
        }
    }

    return { clusters, noise };
}

function calculateClusterLighting(clusters: number[][], points: { position: number[]; node: any }[]) {
    return clusters.map((cluster, index) => {
        if (cluster.length === 0) return null;

        const centroid = [0, 0, 0];
        cluster.forEach(pointIndex => {
            const pos = points[pointIndex].position;
            centroid[0] += pos[0];
            centroid[1] += pos[1];
            centroid[2] += pos[2];
        });

        centroid[0] /= cluster.length;
        centroid[1] /= cluster.length;
        centroid[2] /= cluster.length;

        let maxRadius = 0;
        cluster.forEach(pointIndex => {
            const distance = calculateDistance(centroid, points[pointIndex].position);
            maxRadius = Math.max(maxRadius, distance);
        });

        return {
            centroid,
            radius: maxRadius + 2,
            nodeCount: cluster.length,
            color: `hsl(${(index * 137.5) % 360}, 80%, 55%)`
        };
    }).filter((c): c is NonNullable<typeof c> => c !== null);
}

type SceneProps = {
    nodes: any[];
    activeNode: string;
    setActive: (key: string) => void;
    onNavigateNext?: () => void;
    onNavigatePrevious?: () => void;
    onToggleTour?: () => void;
    onResetToOverview?: () => void;
    onNavigateToIndex?: (index: number) => void;
    onCameraTargetChange?: (target: { position: number[]; lookAt: number[] } | null) => void;
    screenPositionRef?: MutableRefObject<{ x: number; y: number } | null>;
};

export function Scene({
    nodes,
    activeNode,
    setActive,
    onNavigateNext,
    onNavigatePrevious,
    onToggleTour,
    onResetToOverview,
    onNavigateToIndex,
    onCameraTargetChange,
    screenPositionRef
}: SceneProps) {
    const { gl, camera } = useThree();

    // Keyboard event handler
    const handleKeyDown = useCallback((event: KeyboardEvent) => {
        switch (event.key) {
            case 'ArrowRight':
            case 'n':
            case 'N':
                event.preventDefault();
                onNavigateNext?.();
                break;
            case 'ArrowLeft':
            case 'p':
            case 'P':
                event.preventDefault();
                onNavigatePrevious?.();
                break;
            case 't':
            case 'T':
                event.preventDefault();
                onToggleTour?.();
                break;
            case 'Escape':
                event.preventDefault();
                onResetToOverview?.();
                break;
            case '1':
            case '2':
            case '3':
            case '4':
            case '5':
            case '6':
            case '7':
            case '8':
            case '9':
                event.preventDefault();
                const index = parseInt(event.key) - 1;
                onNavigateToIndex?.(index);
                break;
        }
    }, [onNavigateNext, onNavigatePrevious, onToggleTour, onResetToOverview, onNavigateToIndex]);

    // Add keyboard listeners
    useEffect(() => {
        const canvas = gl.domElement;
        canvas.tabIndex = 0;

        const handleFocus = () => {
            window.addEventListener('keydown', handleKeyDown);
        };

        const handleBlur = () => {
            window.removeEventListener('keydown', handleKeyDown);
        };

        window.addEventListener('keydown', handleKeyDown);
        canvas.addEventListener('focus', handleFocus);
        canvas.addEventListener('blur', handleBlur);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            canvas.removeEventListener('focus', handleFocus);
            canvas.removeEventListener('blur', handleBlur);
        };
    }, [gl.domElement, handleKeyDown]);

    // Process nodes
    const adjustedNodes = useMemo(() => {
        const uniqueNodes = nodes.filter((node, index, self) => {
            if (!node?.reduced_embedding || !Array.isArray(node.reduced_embedding)) return false;
            const currentKey = node.reduced_embedding.map((p: any) => {
                let val = typeof p === 'string' ? parseFloat(p) : Number(p);
                return isNaN(val) ? 0 : val;
            }).map(String).join("-");
            return self.findIndex((n: any) => {
                if (!n?.reduced_embedding || !Array.isArray(n.reduced_embedding)) return false;
                const compareKey = n.reduced_embedding.map((p: any) => {
                    let val = typeof p === 'string' ? parseFloat(p) : Number(p);
                    return isNaN(val) ? 0 : val;
                }).map(String).join("-");
                return compareKey === currentKey;
            }) === index;
        });

        return avoidCollisions(uniqueNodes);
    }, [nodes]);

    // Stable key for a node, independent of position
    const getNodeKey = useCallback((node: any): string => {
        return node.concepts?.slice().sort().join("|") ?? "";
    }, []);

    // Find active node position
    const activeNodePosition = useMemo(() => {
        const activeNodeData = adjustedNodes.find((n: any) => getNodeKey(n) === activeNode);
        return activeNodeData ? safeParseEmbedding(activeNodeData.reduced_embedding, SCENE_SCALE) : null;
    }, [adjustedNodes, activeNode, getNodeKey]);

    // Project active node position to screen coordinates
    useFrame(() => {
        if (!screenPositionRef) return;
        if (!activeNodePosition) {
            screenPositionRef.current = null;
            return;
        }
        const vec = new THREE.Vector3(activeNodePosition[0], activeNodePosition[1], activeNodePosition[2]);
        vec.project(camera);
        const halfWidth = gl.domElement.clientWidth / 2;
        const halfHeight = gl.domElement.clientHeight / 2;
        screenPositionRef.current = {
            x: vec.x * halfWidth + halfWidth,
            y: -(vec.y * halfHeight) + halfHeight,
        };
    });

    // Calculate camera target and notify parent
    useEffect(() => {
        if (!activeNodePosition || adjustedNodes.length === 0) {
            onCameraTargetChange?.(null);
            return;
        }

        const clusterDistance = 4 * SCENE_SCALE;
        const nearbyPlanets = adjustedNodes.filter((node: any) => {
            const nodePos = safeParseEmbedding(node.reduced_embedding, SCENE_SCALE);
            const distance = calculateDistance(activeNodePosition, nodePos);
            return distance <= clusterDistance;
        });

        if (nearbyPlanets.length === 0) {
            onCameraTargetChange?.(null);
            return;
        }

        const selectedPlanetPos = [...activeNodePosition];
        let maxRadius = 0;
        nearbyPlanets.forEach((node: any) => {
            const pos = safeParseEmbedding(node.reduced_embedding, SCENE_SCALE);
            const distance = calculateDistance(selectedPlanetPos, pos);
            maxRadius = Math.max(maxRadius, distance);
        });

        const sphereRadius = Math.max(maxRadius + 2 * SCENE_SCALE, 6 * SCENE_SCALE);
        const fov = 75;
        const fovRadians = (fov * Math.PI) / 180;
        const optimalDistance = sphereRadius / Math.tan(fovRadians / 2) * 1.2;

        const elevation = Math.PI / 4;
        const azimuth = Math.PI / 6;

        const cameraPosition = [
            selectedPlanetPos[0] + Math.cos(elevation) * Math.sin(azimuth) * optimalDistance,
            selectedPlanetPos[1] + Math.sin(elevation) * optimalDistance,
            selectedPlanetPos[2] + Math.cos(elevation) * Math.cos(azimuth) * optimalDistance
        ];

        onCameraTargetChange?.({
            lookAt: selectedPlanetPos,
            position: cameraPosition
        });
    }, [activeNodePosition, adjustedNodes, onCameraTargetChange]);

    // Cluster lights
    const clusterLights = useMemo(() => {
        if (adjustedNodes.length < 2) return [];

        const points = adjustedNodes.map((node: any) => ({
            position: safeParseEmbedding(node.reduced_embedding, SCENE_SCALE),
            node
        }));

        const clusterDistance = 4 * SCENE_SCALE;
        const { clusters } = dbscan3D(points, clusterDistance, 2);
        return calculateClusterLighting(clusters, points);
    }, [adjustedNodes]);


    return (
        <>
            <AmbientLighting />
            <ClusterLights clusters={clusterLights} />

            {/* Planets */}
            {adjustedNodes.map((node: any, idx: number) => {
                const safeEmbedding = safeParseEmbedding(node.reduced_embedding, SCENE_SCALE);
                const keyString = getNodeKey(node);
                const isSelected = activeNode === keyString;
                const conceptDistance = 8 * SCENE_SCALE;
                const isNearActive = activeNodePosition ?
                    calculateDistance(safeEmbedding, activeNodePosition) <= conceptDistance : false;
                const shouldShowConcepts = isSelected || isNearActive;
                const textureIndex = getTextureIndexForPlanet(safeEmbedding, textures.length);
                const clusterIndex = node.group_id ?? idx;

                return (
                    <Fragment key={`${keyString}-${idx}`}>
                        <Planet
                            texturePath={textures[textureIndex]}
                            position={safeEmbedding}
                            concepts={node.concepts}
                            onClick={() => setActive(keyString)}
                            isSelected={isSelected}
                            shouldShowConcepts={shouldShowConcepts}
                            scaleSize={SCENE_SCALE}
                            clusterIndex={clusterIndex}
                        />
                    </Fragment>
                );
            })}
        </>
    );
}
