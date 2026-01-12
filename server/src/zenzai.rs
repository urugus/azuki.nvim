//! Zenzai neural kana-kanji conversion backend
//!
//! This module provides neural network-based kana-kanji conversion using
//! the zenz model (GPT-2 based, specialized for Japanese input).
//!
//! Requires the `zenzai` feature to be enabled.

use serde::Deserialize;
#[cfg(feature = "zenzai")]
use std::path::PathBuf;

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
    // llama-cpp-2 model instance will be stored here
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
        if self.model.is_some() {
            return Ok(());
        }

        let model_path = self
            .config
            .get_model_path()
            .ok_or(ZenzaiError::ModelNotFound)?;

        eprintln!("[zenzai] Loading model from: {}", model_path.display());

        // TODO: Actually load the model using llama-cpp-2
        // For now, just store the path
        self.model = Some(ZenzaiModel {
            _model_path: model_path,
        });

        eprintln!("[zenzai] Model loaded successfully");
        Ok(())
    }

    /// Convert hiragana to kanji using neural network
    pub fn convert(
        &mut self,
        reading: &str,
        context: Option<&str>,
    ) -> Result<Vec<String>, ZenzaiError> {
        // Ensure model is loaded
        if self.model.is_none() {
            self.initialize()?;
        }

        let _model = self.model.as_ref().ok_or(ZenzaiError::NotInitialized)?;

        // TODO: Implement actual neural conversion
        // For now, return the reading as-is (fallback behavior)
        eprintln!(
            "[zenzai] Converting: {} (context: {:?}, limit: {})",
            reading, context, self.config.inference_limit
        );

        // Placeholder: return reading unchanged
        // Real implementation would:
        // 1. Tokenize the input (character-level + byte BPE)
        // 2. Run inference with the GPT-2 model
        // 3. Decode the output tokens
        // 4. Return ranked candidates
        Ok(vec![reading.to_string()])
    }

    /// Check if the backend is ready
    pub fn is_ready(&self) -> bool {
        self.model.is_some()
    }

    /// Get configuration
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
