//! Zenzai neural kana-kanji conversion backend
//!
//! This module provides neural network-based kana-kanji conversion using
//! the zenz model (GPT-2 based, specialized for Japanese input).
//!
//! Requires the `zenzai` feature to be enabled.
//!
//! ## zenz-v3 Prompt Format
//!
//! The model uses special Unicode characters as delimiters:
//! - `\u{EE02}`: Context prefix (optional)
//! - `\u{EE00}`: Input reading start
//! - `\u{EE01}`: Output start
//! - `</s>`: End of sequence
//!
//! Format: `\u{EE02}<context>\u{EE00}<hiragana>\u{EE01}<output></s>`

use serde::Deserialize;
#[cfg(feature = "zenzai")]
use std::path::PathBuf;

// zenz-v3 special tokens (Unicode Private Use Area)
#[cfg(feature = "zenzai")]
const ZENZ_INPUT_START: char = '\u{EE00}';
#[cfg(feature = "zenzai")]
const ZENZ_OUTPUT_START: char = '\u{EE01}';
#[cfg(feature = "zenzai")]
const ZENZ_CONTEXT: char = '\u{EE02}';

/// Zenzai configuration
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields are used when zenzai feature is enabled
pub struct ZenzaiConfig {
    /// Enable Zenzai neural conversion
    #[serde(default)]
    pub enabled: bool,

    /// Path to the GGUF model file
    #[serde(default)]
    pub model_path: Option<String>,

    /// Maximum inference iterations (higher = better accuracy, slower)
    #[serde(default = "default_inference_limit")]
    pub inference_limit: u32,

    /// Enable contextual conversion (uses previous text for better results)
    #[serde(default)]
    pub contextual: bool,
}

fn default_inference_limit() -> u32 {
    10
}

impl Default for ZenzaiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            inference_limit: default_inference_limit(),
            contextual: false,
        }
    }
}

impl ZenzaiConfig {
    /// Check if Zenzai is properly configured and can be used
    #[cfg(feature = "zenzai")]
    pub fn is_usable(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // Check if model file exists
        if let Some(ref path) = self.model_path {
            PathBuf::from(path).exists()
        } else {
            // Try default paths
            default_model_paths().iter().any(|p| p.exists())
        }
    }

    /// Check if Zenzai is properly configured and can be used (stub for non-zenzai builds)
    #[cfg(not(feature = "zenzai"))]
    #[allow(dead_code)] // Used in tests
    pub fn is_usable(&self) -> bool {
        // Without zenzai feature, it's never usable
        false
    }

    /// Get the model path, checking default locations if not specified
    #[cfg(feature = "zenzai")]
    pub fn get_model_path(&self) -> Option<PathBuf> {
        if let Some(ref path) = self.model_path {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Search default paths
        default_model_paths().into_iter().find(|p| p.exists())
    }
}

/// Default paths to search for the Zenzai model
#[cfg(feature = "zenzai")]
pub fn default_model_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG data home
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        paths.push(PathBuf::from(&data_home).join("azuki/models/zenz-v3.1-small.gguf"));
        paths.push(PathBuf::from(&data_home).join("azuki/models/zenz.gguf"));
    }

    // Home directory fallback
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".local/share/azuki/models/zenz-v3.1-small.gguf"));
        paths.push(PathBuf::from(&home).join(".local/share/azuki/models/zenz.gguf"));
        paths.push(PathBuf::from(&home).join(".azuki/models/zenz-v3.1-small.gguf"));
        paths.push(PathBuf::from(&home).join(".azuki/models/zenz.gguf"));
    }

    paths
}

/// Zenzai conversion backend
#[cfg(feature = "zenzai")]
pub struct ZenzaiBackend {
    config: ZenzaiConfig,
    // Model will be loaded lazily
    model: Option<ZenzaiModel>,
}

#[cfg(feature = "zenzai")]
struct ZenzaiModel {
    model: llama_cpp_2::model::LlamaModel,
    _model_path: PathBuf,
}

#[cfg(feature = "zenzai")]
impl ZenzaiBackend {
    /// Create a new Zenzai backend with the given configuration
    pub fn new(config: ZenzaiConfig) -> Self {
        Self {
            config,
            model: None,
        }
    }

    /// Initialize the model (lazy loading)
    pub fn initialize(&mut self) -> Result<(), ZenzaiError> {
        use llama_cpp_2::model::params::LlamaModelParams;
        use llama_cpp_2::model::LlamaModel;

        if self.model.is_some() {
            return Ok(());
        }

        let model_path = self
            .config
            .get_model_path()
            .ok_or(ZenzaiError::ModelNotFound)?;

        eprintln!("[zenzai] Loading model from: {}", model_path.display());

        // Initialize llama.cpp backend
        let backend = llama_cpp_2::llama_backend::LlamaBackend::init()
            .map_err(|e| ZenzaiError::LoadError(format!("Failed to init backend: {}", e)))?;

        // Configure model parameters
        let model_params = LlamaModelParams::default();

        // Load the model
        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
            .map_err(|e| ZenzaiError::LoadError(format!("Failed to load model: {}", e)))?;

        self.model = Some(ZenzaiModel {
            model,
            _model_path: model_path,
        });

        eprintln!("[zenzai] Model loaded successfully");
        Ok(())
    }

    /// Build prompt for zenz-v3 model
    fn build_prompt(&self, reading: &str, context: Option<&str>) -> String {
        let mut prompt = String::new();

        // Add context if provided (zenz-v3 format: context comes first)
        if let Some(ctx) = context {
            if !ctx.is_empty() {
                prompt.push(ZENZ_CONTEXT);
                prompt.push_str(ctx);
            }
        }

        // Add input reading
        prompt.push(ZENZ_INPUT_START);
        prompt.push_str(reading);

        // Add output marker (model will generate after this)
        prompt.push(ZENZ_OUTPUT_START);

        prompt
    }

    /// Convert hiragana to kanji using neural network
    pub fn convert(
        &mut self,
        reading: &str,
        context: Option<&str>,
    ) -> Result<Vec<String>, ZenzaiError> {
        use llama_cpp_2::context::params::LlamaContextParams;
        use llama_cpp_2::llama_batch::LlamaBatch;
        use llama_cpp_2::token::LlamaToken;

        // Ensure model is loaded
        if self.model.is_none() {
            self.initialize()?;
        }

        let zenzai_model = self.model.as_ref().ok_or(ZenzaiError::NotInitialized)?;

        eprintln!(
            "[zenzai] Converting: {} (context: {:?}, limit: {})",
            reading, context, self.config.inference_limit
        );

        // Build the prompt
        let prompt = self.build_prompt(reading, context);
        eprintln!("[zenzai] Prompt: {:?}", prompt);

        // Create context for inference
        let ctx_params = LlamaContextParams::default().with_n_ctx(std::num::NonZeroU32::new(512));

        let mut ctx = zenzai_model
            .model
            .new_context(
                &llama_cpp_2::llama_backend::LlamaBackend::init().map_err(|e| {
                    ZenzaiError::InferenceError(format!("Backend init failed: {}", e))
                })?,
                ctx_params,
            )
            .map_err(|e| ZenzaiError::InferenceError(format!("Context creation failed: {}", e)))?;

        // Tokenize the prompt
        let tokens = zenzai_model
            .model
            .str_to_token(&prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| ZenzaiError::InferenceError(format!("Tokenization failed: {}", e)))?;

        eprintln!("[zenzai] Input tokens: {}", tokens.len());

        // Create batch and add tokens
        let mut batch = LlamaBatch::new(512, 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch
                .add(token, i as i32, &[0], is_last)
                .map_err(|e| ZenzaiError::InferenceError(format!("Batch add failed: {}", e)))?;
        }

        // Decode the initial prompt
        ctx.decode(&mut batch)
            .map_err(|e| ZenzaiError::InferenceError(format!("Initial decode failed: {}", e)))?;

        // Generate tokens (greedy decoding)
        let mut output_tokens: Vec<LlamaToken> = Vec::new();
        let max_tokens = self.config.inference_limit as usize * 10; // Allow reasonable output length
        let mut n_cur = tokens.len();

        // Get special token IDs for stopping
        let eos_token = zenzai_model.model.token_eos();

        for _ in 0..max_tokens {
            // Get logits for the last token
            let logits = ctx.get_logits_ith((n_cur - 1) as i32);

            // Simple greedy sampling: pick the token with highest logit
            let mut best_token = LlamaToken::new(0);
            let mut best_logit = f32::NEG_INFINITY;

            for (token_id, &logit) in logits.iter().enumerate() {
                if logit > best_logit {
                    best_logit = logit;
                    best_token = LlamaToken::new(token_id as i32);
                }
            }

            // Check for end of sequence
            if best_token == eos_token {
                break;
            }

            // Decode the token to check for special markers
            let token_str = zenzai_model
                .model
                .token_to_str(best_token, llama_cpp_2::model::Special::Tokenize)
                .unwrap_or_default();

            // Stop if we hit the input start marker (shouldn't happen, but safety check)
            if token_str.contains(ZENZ_INPUT_START) {
                break;
            }

            output_tokens.push(best_token);

            // Prepare next batch
            batch.clear();
            batch
                .add(best_token, n_cur as i32, &[0], true)
                .map_err(|e| ZenzaiError::InferenceError(format!("Batch add failed: {}", e)))?;

            // Decode
            ctx.decode(&mut batch)
                .map_err(|e| ZenzaiError::InferenceError(format!("Decode failed: {}", e)))?;

            n_cur += 1;
        }

        // Decode output tokens to string
        let mut output = String::new();
        for token in &output_tokens {
            if let Ok(s) = zenzai_model
                .model
                .token_to_str(*token, llama_cpp_2::model::Special::Tokenize)
            {
                output.push_str(&s);
            }
        }

        // Clean up the output (remove </s> if present)
        let output = output.trim_end_matches("</s>").to_string();

        eprintln!("[zenzai] Output: {}", output);

        // Return the result (single candidate for now)
        if output.is_empty() {
            // Fallback to reading if no output
            Ok(vec![reading.to_string()])
        } else {
            Ok(vec![output, reading.to_string()])
        }
    }

    /// Check if the backend is ready
    pub fn is_ready(&self) -> bool {
        self.model.is_some()
    }

    /// Get configuration
    #[allow(dead_code)]
    pub fn config(&self) -> &ZenzaiConfig {
        &self.config
    }
}

/// Zenzai-specific errors
#[cfg(feature = "zenzai")]
#[derive(Debug)]
pub enum ZenzaiError {
    /// Model file not found
    ModelNotFound,
    /// Backend not initialized
    NotInitialized,
    /// Model loading failed
    #[allow(dead_code)]
    LoadError(String),
    /// Inference failed
    #[allow(dead_code)]
    InferenceError(String),
}

#[cfg(feature = "zenzai")]
impl std::fmt::Display for ZenzaiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZenzaiError::ModelNotFound => write!(f, "Zenzai model file not found"),
            ZenzaiError::NotInitialized => write!(f, "Zenzai backend not initialized"),
            ZenzaiError::LoadError(msg) => write!(f, "Failed to load Zenzai model: {}", msg),
            ZenzaiError::InferenceError(msg) => write!(f, "Zenzai inference failed: {}", msg),
        }
    }
}

#[cfg(feature = "zenzai")]
impl std::error::Error for ZenzaiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ZenzaiConfig::default();
        assert!(!config.enabled);
        assert!(config.model_path.is_none());
        assert_eq!(config.inference_limit, 10);
        assert!(!config.contextual);
    }

    #[test]
    fn test_config_not_usable_when_disabled() {
        let config = ZenzaiConfig {
            enabled: false,
            model_path: Some("/some/path".to_string()),
            ..Default::default()
        };
        assert!(!config.is_usable());
    }
}
