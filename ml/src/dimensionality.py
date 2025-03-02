
from sklearn.decomposition import PCA
from sklearn.manifold import TSNE
from sklearn.cluster import KMeans
import numpy as np
from sklearn.metrics.pairwise import cosine_similarity

def cluster_embeddings(embeddings, n_clusters=3):

    embeddings_array = np.array(embeddings)
    
    kmeans = KMeans(n_clusters=n_clusters, random_state=42)
    cluster_labels = kmeans.fit_predict(embeddings_array)
    
    return cluster_labels


def reduce_to_3d(embeddings, method='pca'):
    if method == 'pca':
        reducer = PCA(n_components=3)
    elif method == 'tsne':
        reducer = TSNE(n_components=3, perplexity=5, random_state=42)
    else:
        raise ValueError("Unsupported method. Use 'pca' or 'tsne'.")
    
    reduced_embeddings = reducer.fit_transform(embeddings)
    return reduced_embeddings



def merge_similar_concepts(concepts, embeddings, similarity_threshold=0.8):
    embeddings_array = np.array(embeddings)
    similarities = cosine_similarity(embeddings_array)
    
    # Lista para almacenar los grupos finales
    merged_groups = []
    processed = set()
    
    for i in range(len(concepts)):
        if i in processed:
            continue
            
        # Encuentra conceptos similares
        similar_indices = [i]
        for j in range(i + 1, len(concepts)):
            if j not in processed and similarities[i][j] > similarity_threshold:
                similar_indices.append(j)
        
        # Agrupa conceptos y calcula embedding promedio
        group_concepts = [concepts[idx]["concept"] for idx in similar_indices]
        group_embeddings = embeddings_array[similar_indices]
        merged_embedding = np.mean(group_embeddings, axis=0)
        
        # Agrega el grupo a la lista final
        merged_groups.append({
            "concepts": group_concepts,
            "embedding": merged_embedding.tolist()
        })
        
        processed.update(similar_indices)
    
    return merged_groups

def clusterConcepts(concepts, embeddings):
    # Obtiene grupos de conceptos similares con sus embeddings promediados
    merged_groups = merge_similar_concepts(concepts, embeddings)
    
    # Extrae solo los embeddings para clustering y reducción
    merged_embeddings = [group["embedding"] for group in merged_groups]
    
    # Aplica clustering
    clusters = cluster_embeddings(merged_embeddings, n_clusters=3)
    
    # Reduce dimensiones para visualización
    reduced_embeddings = reduce_to_3d(merged_embeddings, method='pca')
    
    # Asigna clusters y embeddings reducidos a cada grupo
    final_groups = []
    for i, group in enumerate(merged_groups):
        final_groups.append({
            "concepts": group["concepts"],
            "reduced_embedding": reduced_embeddings[i].tolist(),
            "cluster": int(clusters[i])
        })
    
    # Imprime resultados por cluster (opcional)
    for cluster_id in range(max(clusters) + 1):
        print(f"\nCluster {cluster_id}:")
        cluster_groups = [g["concepts"] for g in final_groups if g["cluster"] == cluster_id]
        print(cluster_groups)
    
    return final_groups

