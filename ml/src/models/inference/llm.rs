use async_trait::async_trait;
use log::info;
use mistralrs::{
    Constraint, GgufModelBuilder, MemoryGpuConfig, Model, PagedAttentionMetaBuilder,
    RequestBuilder, TextMessageRole,
};

use super::config::InferenceConfig;
use super::traits::{GenerationParams, InferenceError, LlmBackend};

pub struct MistralRsLlm {
    model: Model,
    model_id: String,
}

impl MistralRsLlm {
    pub async fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        let mut builder = GgufModelBuilder::new(
            &config.llm_model,
            config.llm_gguf_files.clone(),
        )
        .with_logging();

        if config.use_gpu {
            if mistralrs::paged_attn_supported() {
                let mem_config = if let Some(util) = config.llm_gpu_utilization {
                    info!("Using GPU utilization-based KV cache sizing: {:.0}%", util * 100.0);
                    MemoryGpuConfig::Utilization(util)
                } else {
                    info!("Using fixed context size for KV cache: {}", config.llm_context_size);
                    MemoryGpuConfig::ContextSize(config.llm_context_size)
                };

                builder = builder
                    .with_paged_attn(|| {
                        PagedAttentionMetaBuilder::default()
                            .with_gpu_memory(mem_config)
                            .build()
                    })
                    .map_err(|e| InferenceError::DeviceError(e.to_string()))?;
            }
        } else {
            builder = builder.with_force_cpu();
        }

        let model = builder
            .build()
            .await
            .map_err(|e| InferenceError::ModelLoadError(e.to_string()))?;

        Ok(Self {
            model,
            model_id: config.llm_model.clone(),
        })
    }
}

#[async_trait]
impl LlmBackend for MistralRsLlm {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        params: &GenerationParams,
    ) -> Result<String, InferenceError> {
        let mut request = RequestBuilder::new()
            .add_message(TextMessageRole::System, system_prompt)
            .add_message(TextMessageRole::User, user_prompt);

        if params.temperature == 0.0 {
            request = request.set_deterministic_sampler();
        } else {
            request = request.set_sampler_temperature(params.temperature as f64);
        }

        if let Some(max_tokens) = params.max_tokens {
            request = request.set_sampler_max_len(max_tokens as usize);
        }

        if let Some(ref schema) = params.json_schema {
            request = request.set_constraint(Constraint::JsonSchema(schema.clone()));
        }

        let response = self
            .model
            .send_chat_request(request)
            .await
            .map_err(|e| InferenceError::InferenceFailed(e.to_string()))?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .cloned()
            .ok_or_else(|| InferenceError::InferenceFailed("Empty response".into()))
    }

    async fn warmup(&self) -> Result<(), InferenceError> {
        info!("Warming up LLM model: {}", self.model_id);
        let request = RequestBuilder::new()
            .add_message(TextMessageRole::User, "Hello")
            .set_deterministic_sampler()
            .set_sampler_max_len(1);

        self.model
            .send_chat_request(request)
            .await
            .map_err(|e| InferenceError::InferenceFailed(e.to_string()))?;
        Ok(())
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
