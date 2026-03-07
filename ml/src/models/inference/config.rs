/// Configuration for local inference backends, loaded from environment variables.
pub struct InferenceConfig {
    pub llm_model: String,
    pub llm_gguf_files: Vec<String>,
    pub embedding_model: String,
    pub use_gpu: bool,
    pub llm_context_size: usize,
    /// GPU memory utilization for paged attention (0.0-1.0). When set, overrides `llm_context_size`.
    pub llm_gpu_utilization: Option<f32>,
    pub embedding_force_cpu: bool,
}

impl InferenceConfig {
    pub fn from_env() -> Self {
        let llm_model = std::env::var("LLM_MODEL_REPO")
            .unwrap_or_else(|_| "bartowski/Phi-3.5-mini-instruct-GGUF".to_string());

        let llm_gguf_files: Vec<String> = std::env::var("LLM_GGUF_FILES")
            .unwrap_or_else(|_| "Phi-3.5-mini-instruct-Q4_K_M.gguf".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let embedding_model = std::env::var("EMBEDDING_MODEL_REPO")
            .unwrap_or_else(|_| "Qwen/Qwen3-Embedding-0.6B".to_string());

        let use_gpu = std::env::var("GPU_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        let llm_context_size = std::env::var("LLM_CONTEXT_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4096);

        let llm_gpu_utilization = std::env::var("LLM_GPU_UTILIZATION")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .or(Some(0.8));

        let embedding_force_cpu = std::env::var("EMBEDDING_ON_CPU")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            llm_model,
            llm_gguf_files,
            embedding_model,
            use_gpu,
            llm_context_size,
            llm_gpu_utilization,
            embedding_force_cpu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_defaults() {
        // Clear any env vars that might interfere
        env::remove_var("LLM_MODEL_REPO");
        env::remove_var("LLM_GGUF_FILES");
        env::remove_var("EMBEDDING_MODEL_REPO");
        env::remove_var("GPU_ENABLED");
        env::remove_var("LLM_CONTEXT_SIZE");
        env::remove_var("LLM_GPU_UTILIZATION");
        env::remove_var("EMBEDDING_ON_CPU");

        let config = InferenceConfig::from_env();

        assert_eq!(config.llm_model, "bartowski/Phi-3.5-mini-instruct-GGUF");
        assert_eq!(config.llm_gguf_files, vec!["Phi-3.5-mini-instruct-Q4_K_M.gguf"]);
        assert_eq!(config.embedding_model, "Qwen/Qwen3-Embedding-0.6B");
        assert!(config.use_gpu);
        assert_eq!(config.llm_context_size, 4096);
        assert_eq!(config.llm_gpu_utilization, Some(0.8));
        assert!(!config.embedding_force_cpu);
    }

    #[test]
    fn test_config_custom_env() {
        env::set_var("LLM_MODEL_REPO", "custom/model");
        env::set_var("EMBEDDING_MODEL_REPO", "custom/embeddings");
        env::set_var("GPU_ENABLED", "false");

        let config = InferenceConfig::from_env();

        assert_eq!(config.llm_model, "custom/model");
        assert_eq!(config.embedding_model, "custom/embeddings");
        assert!(!config.use_gpu);

        // Cleanup
        env::remove_var("LLM_MODEL_REPO");
        env::remove_var("EMBEDDING_MODEL_REPO");
        env::remove_var("GPU_ENABLED");
    }

    #[test]
    fn test_config_gguf_files_parsing() {
        env::set_var("LLM_GGUF_FILES", "file1.gguf, file2.gguf, file3.gguf");

        let config = InferenceConfig::from_env();

        assert_eq!(
            config.llm_gguf_files,
            vec!["file1.gguf", "file2.gguf", "file3.gguf"]
        );

        // Cleanup
        env::remove_var("LLM_GGUF_FILES");
    }
}
