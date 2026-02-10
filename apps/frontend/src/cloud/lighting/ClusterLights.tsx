import { SCENE_SCALE } from "../hooks/useSceneScale";

type ClusterLight = {
  centroid: number[];
  radius: number;
  nodeCount: number;
  color: string;
};

type Props = {
  clusters: ClusterLight[];
};

export function ClusterLights({ clusters }: Props) {
  return (
    <>
      {clusters.map((cluster, index) => (
        <group key={`cluster-light-${index}`}>
          <pointLight
            position={cluster.centroid as [number, number, number]}
            intensity={Math.min(cluster.nodeCount * 3, 10)}
            distance={cluster.radius * 2}
            color={cluster.color}
            decay={1}
          />
          <pointLight
            position={[
              cluster.centroid[0],
              cluster.centroid[1] + 2 * SCENE_SCALE,
              cluster.centroid[2]
            ]}
            intensity={Math.min(cluster.nodeCount * 1.5, 5)}
            distance={cluster.radius * 1.5}
            color={cluster.color}
            decay={1}
          />
        </group>
      ))}
    </>
  );
}
