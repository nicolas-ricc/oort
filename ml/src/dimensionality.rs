use crate::concepts::Concept;
use crate::embeddings::Embedding;
use crate::error::ApiError;
use log::info;
use ndarray::{Array2, ArrayView1};
use ndarray_stats::QuantileExt;
use ndarray_linalg::Norm;
use linfa::prelude::*;
use linfa_clustering::KMeans;
use linfa_reduction::Pca;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptGroup {
    pub concepts: Vec<String>,
    pub reduced_embedding: Vec<f32>,
    pub cluster: usize,
}

// Helper function to calculate cosine similarity
fn cosine_similarity(a: ArrayView1<f32>, b: ArrayView1<f32>) -> f32 {
    let norm_a = a.norm_l2();
    let norm_b = b.norm_l2();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    // Use dot product from BLAS
    a.dot(&b) / (norm_a * norm_b)
}

// Merge similar concepts based on cosine similarity
pub fn merge_similar_concepts(
    concepts: &[Concept],
    embeddings: &[Embedding],
    similarity_threshold: f32,
) -> Result<Vec<(Vec<String>, Embedding)>, ApiError> {
    if concepts.is_empty() || embeddings.is_empty() {
        return Err(ApiError::InternalError("Empty concepts or embeddings".to_string()));
    }
    
    if concepts.len() != embeddings.len() {
        return Err(ApiError::InternalError(format!(
            "Concepts length ({}) does not match embeddings length ({})",
            concepts.len(), embeddings.len()
        )));
    }
    
    let mut merged_groups = Vec::new();
    let mut processed = HashSet::new();
    
    for i in 0..concepts.len() {
        if processed.contains(&i) {
            continue;
        }
        
        // Find similar concepts
        let mut similar_indices = vec![i];
        for j in (i + 1)..concepts.len() {
            if !processed.contains(&j) {
                let similarity = cosine_similarity(
                    embeddings[i].view(),
                    embeddings[j].view()
                );
                
                if similarity > similarity_threshold {
                    similar_indices.push(j);
                }
            }
        }
        
        // Extract concept strings from groups
        let group_concepts = similar_indices.iter()
            .map(|&idx| concepts[idx].concept.clone())
            .collect::<Vec<_>>();
        
        // Calculate average embedding
        let mut avg_embedding = embeddings[i].clone();
        if similar_indices.len() > 1 {
            avg_embedding.fill(0.0);
            for &idx in &similar_indices {
                avg_embedding += &embeddings[idx];
            }
            avg_embedding /= similar_indices.len() as f32;
        }
        
        // Add group to merged results
        merged_groups.push((group_concepts, avg_embedding));
        
        // Mark indices as processed
        for &idx in &similar_indices {
            processed.insert(idx);
        }
    }
    
    Ok(merged_groups)
}

// Cluster embeddings
pub fn cluster_embeddings(
    embeddings: &[Embedding],
    n_clusters: usize,
) -> Result<Vec<usize>, ApiError> {
    if embeddings.is_empty() {
        return Err(ApiError::InternalError("Empty embeddings".to_string()));
    }
    
    // Convert to dataset with f64 precision
    let n_features = embeddings[0].len();
    let mut data = Array2::zeros((embeddings.len(), n_features));
    
    for (i, embedding) in embeddings.iter().enumerate() {
        // Convert f32 to f64 for each element
        for (j, &val) in embedding.iter().enumerate() {
            data[[i, j]] = val as f64;
        }
    }
    
    let dataset = Dataset::from(data);
    
    // Apply K-means clustering
    let kmeans = KMeans::params(n_clusters)
        .max_n_iterations(100)
        .tolerance(1e-5)
        .fit(&dataset)
        .map_err(|e| {
            ApiError::DimensionalityError(format!("K-Means error: {}", e))
        })?;
    
    // Get cluster assignments
    let predictions = kmeans.predict(dataset);
    Ok(predictions.targets.iter().map(|&x| x as usize).collect())
}

// Reduce dimensionality to 3D
pub fn reduce_to_3d(embeddings: &[Embedding]) -> Result<Vec<[f32; 3]>, ApiError> {
    if embeddings.is_empty() {
        return Err(ApiError::InternalError("Empty embeddings".to_string()));
    }
    
    // Convert to dataset with f64 precision
    let n_features = embeddings[0].len();
    let mut data = Array2::zeros((embeddings.len(), n_features));
    
    for (i, embedding) in embeddings.iter().enumerate() {
        // Convert f32 to f64 for each element
        for (j, &val) in embedding.iter().enumerate() {
            data[[i, j]] = val as f64;
        }
    }
    
    let dataset = Dataset::from(data);
    
    // Apply PCA to reduce to 3 dimensions
    let n_components = 3;
    let pca = Pca::params(n_components)
        .fit(&dataset)
        .map_err(|e| {
            ApiError::DimensionalityError(format!("PCA error: {}", e))
        })?;
    
    let transformed = pca.transform(dataset);
    
    // Convert back to f32 for our return type
    let reduced: Vec<[f32; 3]> = transformed
        .records
        .rows()
        .into_iter()
        .map(|row| {
            [row[0] as f32, row[1] as f32, row[2] as f32]
        })
        .collect();
    
    Ok(reduced)
}

pub fn cluster_concepts(
    concepts: &[Concept],
    embeddings: &[Embedding],
) -> Result<Vec<ConceptGroup>, ApiError> {
    // Merge similar concepts
    let merged_groups = merge_similar_concepts(concepts, embeddings, 0.8)?;
    
    // Extract embeddings for clustering
    let merged_embeddings: Vec<Embedding> = merged_groups.iter()
        .map(|(_, embedding)| embedding.clone())
        .collect();
    
    // Apply clustering
    let n_clusters = 3.min(merged_embeddings.len());
    let clusters = cluster_embeddings(&merged_embeddings, n_clusters)?;
    
    // Reduce dimensions for visualization
    let reduced_embeddings = reduce_to_3d(&merged_embeddings)?;
    
    // Create final groups
    let mut final_groups = Vec::new();
    for (i, ((concepts, _), reduced)) in merged_groups.iter().zip(reduced_embeddings).enumerate() {
        final_groups.push(ConceptGroup {
            concepts: concepts.clone(),
            reduced_embedding: reduced.to_vec(),
            cluster: clusters[i],
        });
    }
    
    // Debug: Print results by cluster
    if let Some(&max_cluster) = clusters.iter().max() {
        for cluster_id in 0..=max_cluster {
            info!("Cluster {}:", cluster_id);
            let cluster_groups: Vec<&Vec<String>> = final_groups.iter()
                .filter(|g| g.cluster == cluster_id)
                .map(|g| &g.concepts)
                .collect();
            info!("{:?}", cluster_groups);
        }
    }
    
    Ok(final_groups)
}