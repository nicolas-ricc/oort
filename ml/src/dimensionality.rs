use crate::error::ApiError;
use crate::models::concepts::Concept;
use crate::models::embeddings::Embedding;
use linfa_reduction::Pca;
use log::info;
use ndarray::ArrayView1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptGroup {
    pub concepts: Vec<String>,
    pub reduced_embedding: Vec<f32>,
    pub connections: Vec<usize>,
    pub importance_score: f32,
    pub group_id: usize,
}

#[derive(Debug, Clone)]
pub struct ForceParams {
    pub attraction_strength: f32,
    pub repulsion_strength: f32,
    pub center_gravity: f32,
    pub damping: f32,
    pub min_distance: f32,
    pub max_velocity: f32,
    pub iterations: usize,
    pub similarity_threshold: f32,
}

impl Default for ForceParams {
    fn default() -> Self {
        Self {
            attraction_strength: 2.0,
            repulsion_strength: 10.0,
            center_gravity: 0.1,
            damping: 0.9,
            min_distance: 3.0,
            max_velocity: 2.0,
            iterations: 150,
            similarity_threshold: 0.7,
        }
    }
}

pub struct MindMapProcessor {
    force_params: ForceParams,
    similarity_matrix: Vec<Vec<f32>>,
    positions: Vec<[f32; 3]>,
    concept_groups: Vec<ConceptGroup>,
}

impl MindMapProcessor {
    pub fn new(force_params: Option<ForceParams>) -> Self {
        Self {
            force_params: force_params.unwrap_or_default(),
            similarity_matrix: Vec::new(),
            positions: Vec::new(),
            concept_groups: Vec::new(),
        }
    }

    pub fn process_concepts(
        &mut self,
        concepts: &[Concept],
        embeddings: &[Embedding],
    ) -> Result<Vec<ConceptGroup>, ApiError> {
        info!(
            "Starting mind map processing for {} concepts",
            concepts.len()
        );

        // Step 1: Merge similar concepts
        let merged_groups = self.merge_similar_concepts(concepts, embeddings)?;

        // Step 2: Extract merged embeddings for processing
        let merged_embeddings: Vec<Embedding> = merged_groups
            .iter()
            .map(|(_, embedding, _, _)| embedding.clone())
            .collect();

        // Step 3: Build similarity matrix (continuous, no threshold)
        self.build_similarity_matrix(&merged_embeddings);

        // Step 4: Run force-directed layout with PCA initialization
        self.run_force_directed_layout(&merged_embeddings)?;

        // Step 5: Build final concept groups
        self.build_concept_groups(&merged_groups);

        Ok(self.concept_groups.clone())
    }

    fn merge_similar_concepts(
        &self,
        concepts: &[Concept],
        embeddings: &[Embedding],
    ) -> Result<Vec<(Vec<String>, Embedding, Vec<f32>, usize)>, ApiError> {
        if concepts.is_empty() || embeddings.is_empty() {
            return Err(ApiError::InternalError(
                "Empty concepts or embeddings".to_string(),
            ));
        }

        if concepts.len() != embeddings.len() {
            return Err(ApiError::InternalError(format!(
                "Concepts length ({}) does not match embeddings length ({})",
                concepts.len(),
                embeddings.len()
            )));
        }

        // Check for zero-dimensional embeddings
        for (i, embedding) in embeddings.iter().enumerate() {
            if embedding.len() == 0 {
                log::error!("Embedding {} has zero dimensions for concept: '{}'", 
                           i, concepts[i].concept);
                return Err(ApiError::InternalError(format!(
                    "Embedding {} has zero dimensions", i
                )));
            }
        }

        // Log embedding dimensions for debugging
        let dimensions: Vec<usize> = embeddings.iter().map(|e| e.len()).collect();
        let unique_dims: std::collections::HashSet<usize> = dimensions.iter().cloned().collect();
        info!("Embedding dimensions: {} unique dimensions found: {:?}", 
              unique_dims.len(), unique_dims);
        
        if unique_dims.len() > 1 {
            log::warn!("Inconsistent embedding dimensions detected");
            for (i, &dim) in dimensions.iter().enumerate() {
                if dim != embeddings[0].len() {
                    log::warn!("Embedding {} has {} dimensions, expected {}", 
                              i, dim, embeddings[0].len());
                }
            }
        }

        // Build similarity matrix first
        let mut similarity_matrix = vec![vec![0.0; concepts.len()]; concepts.len()];
        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                let similarity = self.cosine_similarity(embeddings[i].view(), embeddings[j].view());
                similarity_matrix[i][j] = similarity;
                similarity_matrix[j][i] = similarity;
            }
        }

        // Use Union-Find to group similar concepts
        let mut parent = (0..concepts.len()).collect::<Vec<_>>();

        fn find(parent: &mut [usize], x: usize) -> usize {
            if parent[x] != x {
                parent[x] = find(parent, parent[x]);
            }
            parent[x]
        }

        fn union(parent: &mut [usize], x: usize, y: usize) {
            let root_x = find(parent, x);
            let root_y = find(parent, y);
            if root_x != root_y {
                parent[root_y] = root_x;
            }
        }

        // Group similar concepts
        for i in 0..concepts.len() {
            for j in (i + 1)..concepts.len() {
                if similarity_matrix[i][j] > self.force_params.similarity_threshold {
                    union(&mut parent, i, j);
                }
            }
        }

        // Collect groups
        let mut groups: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();
        for i in 0..concepts.len() {
            let root = find(&mut parent, i);
            groups.entry(root).or_insert_with(Vec::new).push(i);
        }

        let mut merged_groups = Vec::new();
        for (root, indices) in &groups {
            let group_concepts: Vec<String> = indices
                .iter()
                .map(|&idx| concepts[idx].concept.clone())
                .collect();

            let group_importances: Vec<f32> = indices
                .iter()
                .map(|&idx| concepts[idx].importance)
                .collect();

            // Average embeddings
            let mut avg_embedding = embeddings[indices[0]].clone();
            if indices.len() > 1 {
                avg_embedding.fill(0.0);
                for &idx in indices {
                    avg_embedding += &embeddings[idx];
                }
                avg_embedding /= indices.len() as f32;
            }

            merged_groups.push((group_concepts, avg_embedding, group_importances, *root));
        }

        info!(
            "Merged {} concepts into {} groups",
            concepts.len(),
            merged_groups.len()
        );

        Ok(merged_groups)
    }

    fn build_similarity_matrix(&mut self, embeddings: &[Embedding]) {
        let n = embeddings.len();
        self.similarity_matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let similarity = self.cosine_similarity(embeddings[i].view(), embeddings[j].view());
                // Store all positive similarities (continuous, no threshold)
                // This preserves gradient information for force-directed layout
                if similarity > 0.0 {
                    self.similarity_matrix[i][j] = similarity;
                    self.similarity_matrix[j][i] = similarity;
                }
            }
        }
    }

    fn run_force_directed_layout(&mut self, embeddings: &[Embedding]) -> Result<(), ApiError> {
        let n = embeddings.len();
        if n == 0 {
            return Err(ApiError::InternalError("No concepts to layout".to_string()));
        }

        // Use PCA to initialize positions from embedding space (deterministic)
        self.positions = self.initialize_pca_positions(embeddings)?;

        let convergence_threshold = 0.001;

        for iteration in 0..self.force_params.iterations {
            let total_energy = self.apply_physics_step();

            if iteration % 50 == 0 {
                info!(
                    "Force-directed iteration: {}/{} (energy: {:.4})",
                    iteration, self.force_params.iterations, total_energy
                );
            }

            // Early termination if system has converged
            if total_energy < convergence_threshold {
                info!(
                    "Force layout converged at iteration {} (energy: {:.6})",
                    iteration, total_energy
                );
                break;
            }
        }

        Ok(())
    }

    fn build_concept_groups(
        &mut self,
        merged_groups: &[(Vec<String>, Embedding, Vec<f32>, usize)],
    ) {
        self.concept_groups.clear();

        info!("Building concept groups: {} merged groups, {} positions",
              merged_groups.len(), self.positions.len());

        // Remap root IDs to compact sequential group IDs
        let unique_roots: Vec<usize> = {
            let mut roots: Vec<usize> = merged_groups.iter().map(|(_, _, _, root)| *root).collect();
            roots.sort();
            roots.dedup();
            roots
        };

        for (i, (concepts, _, importances, root)) in merged_groups.iter().enumerate() {
            if i >= self.positions.len() {
                log::error!("Position index {} out of bounds for positions array of length {}",
                           i, self.positions.len());
                continue;
            }

            let connections = self.find_connections(i);
            let importance_score = self.calculate_importance(i, concepts, importances);
            let group_id = unique_roots.iter().position(|r| r == root).unwrap_or(0);

            self.concept_groups.push(ConceptGroup {
                concepts: concepts.clone(),
                reduced_embedding: self.positions[i].to_vec(),
                connections,
                importance_score,
                group_id,
            });
        }

        info!("Successfully built {} concept groups", self.concept_groups.len());
    }

    fn find_connections(&self, index: usize) -> Vec<usize> {
        self.similarity_matrix[index]
            .iter()
            .enumerate()
            .filter_map(|(i, &similarity)| {
                if i != index && similarity > 0.0 {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    fn calculate_importance(&self, index: usize, concepts: &[String], importances: &[f32]) -> f32 {
        let connection_count = self.similarity_matrix[index]
            .iter()
            .filter(|&&sim| sim > 0.0)
            .count() as f32;
        let concept_count = concepts.len() as f32;

        let avg_nlp_importance = if importances.is_empty() {
            0.5
        } else {
            importances.iter().sum::<f32>() / importances.len() as f32
        };

        (avg_nlp_importance * 0.4 + connection_count * 0.4 + concept_count * 0.2).max(0.1)
    }

    // Physics helper methods
    fn initialize_pca_positions(&self, embeddings: &[Embedding]) -> Result<Vec<[f32; 3]>, ApiError> {
        use linfa::traits::Fit;
        use linfa::traits::Predict;
        use linfa::DatasetBase;
        use ndarray::Array2 as Array2_64;

        let n = embeddings.len();
        if n == 0 {
            return Err(ApiError::InternalError("No embeddings for PCA".to_string()));
        }

        // Handle degenerate case: 1-2 embeddings can't do meaningful PCA
        if n <= 2 {
            let mut positions = Vec::with_capacity(n);
            for i in 0..n {
                positions.push([(i as f32) * 3.0 - 1.5, 0.0, 0.0]);
            }
            return Ok(positions);
        }

        let dim = embeddings[0].len();

        // linfa PCA requires f64, convert from f32 embeddings
        let flat: Vec<f64> = embeddings.iter().flat_map(|e| e.iter().map(|&v| v as f64)).collect();
        let matrix = Array2_64::<f64>::from_shape_vec((n, dim), flat).map_err(|e| {
            ApiError::InternalError(format!("Failed to build embedding matrix: {}", e))
        })?;

        let dataset = DatasetBase::from(matrix);

        // PCA to 3 components
        let pca = Pca::params(3).fit(&dataset).map_err(|e| {
            ApiError::InternalError(format!("PCA fitting failed: {}", e))
        })?;

        let projected = pca.predict(&dataset);

        // Scale to [-5, 5] range
        let mut min_vals = [f64::MAX; 3];
        let mut max_vals = [f64::MIN; 3];
        for row in projected.rows() {
            for (d, &val) in row.iter().enumerate() {
                if d < 3 {
                    min_vals[d] = min_vals[d].min(val);
                    max_vals[d] = max_vals[d].max(val);
                }
            }
        }

        let positions: Vec<[f32; 3]> = projected
            .rows()
            .into_iter()
            .map(|row| {
                let mut pos = [0.0f32; 3];
                for d in 0..3 {
                    let range = max_vals[d] - min_vals[d];
                    if range > 1e-6 {
                        pos[d] = ((row[d] - min_vals[d]) / range * 10.0 - 5.0) as f32;
                    }
                }
                pos
            })
            .collect();

        info!("PCA initialized {} positions in 3D space", positions.len());
        Ok(positions)
    }

    /// Returns total kinetic energy for convergence detection
    fn apply_physics_step(&mut self) -> f32 {
        let mut new_positions = self.positions.clone();
        let mut total_energy = 0.0f32;

        for i in 0..self.positions.len() {
            let mut velocity = [0.0; 3];

            // Attraction forces (continuous similarity as weight)
            for j in 0..self.positions.len() {
                if i != j && self.similarity_matrix[i][j] > 0.0 {
                    let direction =
                        self.subtract_and_normalize(self.positions[j], self.positions[i]);
                    let force =
                        self.similarity_matrix[i][j] * self.force_params.attraction_strength;
                    velocity = self.add_scaled(velocity, direction, force);
                }
            }

            // Universal repulsion forces (all pairs, inverse-square)
            for j in 0..self.positions.len() {
                if i != j {
                    let distance = self.calculate_distance(self.positions[i], self.positions[j]);
                    let direction =
                        self.subtract_and_normalize(self.positions[i], self.positions[j]);
                    let force =
                        self.force_params.repulsion_strength / (distance * distance + 0.01);
                    velocity = self.add_scaled(velocity, direction, force);
                }
            }

            // Center gravity (weak, prevents drift)
            let to_center = self.scale_vector(self.positions[i], -self.force_params.center_gravity);
            velocity = self.add_vectors(velocity, to_center);

            // Apply damping and limits
            velocity = self.scale_vector(velocity, self.force_params.damping);
            velocity = self.clamp_magnitude(velocity, self.force_params.max_velocity);

            // Track kinetic energy for convergence
            total_energy += velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];

            new_positions[i] = self.add_vectors(self.positions[i], velocity);
        }

        self.positions = new_positions;
        total_energy
    }

    fn cosine_similarity(&self, a: ArrayView1<f32>, b: ArrayView1<f32>) -> f32 {
        use ndarray_linalg::Norm;
        let norm_a = a.norm_l2();
        let norm_b = b.norm_l2();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        a.dot(&b) / (norm_a * norm_b)
    }

    // Vector math helpers (same as before)
    fn subtract_and_normalize(&self, a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let diff = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let mag = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
        if mag > 0.0001 {
            [diff[0] / mag, diff[1] / mag, diff[2] / mag]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    fn add_scaled(&self, a: [f32; 3], b: [f32; 3], scale: f32) -> [f32; 3] {
        [
            a[0] + b[0] * scale,
            a[1] + b[1] * scale,
            a[2] + b[2] * scale,
        ]
    }

    fn add_vectors(&self, a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
    }

    fn scale_vector(&self, v: [f32; 3], scale: f32) -> [f32; 3] {
        [v[0] * scale, v[1] * scale, v[2] * scale]
    }

    fn calculate_distance(&self, a: [f32; 3], b: [f32; 3]) -> f32 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn clamp_magnitude(&self, v: [f32; 3], max_mag: f32) -> [f32; 3] {
        let mag = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if mag > max_mag && mag > 0.0001 {
            let scale = max_mag / mag;
            [v[0] * scale, v[1] * scale, v[2] * scale]
        } else {
            v
        }
    }
}