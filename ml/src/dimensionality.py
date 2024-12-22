
from sklearn.decomposition import PCA
from sklearn.manifold import TSNE
import numpy as np

def reduce_to_3d(embeddings, method='pca'):
    if method == 'pca':
        reducer = PCA(n_components=1)
    elif method == 'tsne':
        reducer = TSNE(n_components=3, perplexity=5, random_state=42)
    else:
        raise ValueError("Unsupported method. Use 'pca' or 'tsne'.")
    
    reduced_embeddings = reducer.fit_transform(embeddings)
    return reduced_embeddings



