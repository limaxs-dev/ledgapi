//! Embedder impls: [`FastembedEmbedder`] for production, [`StubEmbedder`]
//! for unit tests (deterministic vectors from text hash).
//!
//! Per spec §13 #15, we do not cache embeddings — fastembed re-encodes
//! each call.

use crate::domain::errors::DomainError;
use crate::domain::ports::Embedder;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

const DIM: usize = 384;

/// Production embedder. Wraps fastembed-rs (synchronous) via
/// `tokio::task::spawn_blocking` so the ONNX inference does not block
/// the Tokio runtime thread.
pub struct FastembedEmbedder {
    model: Arc<StdMutex<fastembed::TextEmbedding>>,
}

impl FastembedEmbedder {
    /// Construct. Triggers model download on first use.
    ///
    /// # Errors
    /// Returns `Err` if the model cannot be loaded (network failure,
    /// invalid cache, etc.). Logs the underlying error at `error` level.
    pub fn new(cache_dir: &str, model_name: &str) -> anyhow::Result<Self> {
        use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

        let parsed_model = match model_name {
            "sentence-transformers/all-MiniLM-L6-v2" | "AllMiniLML6V2" => {
                EmbeddingModel::AllMiniLML6V2
            }
            other => {
                anyhow::bail!("unsupported embed model: {other}");
            }
        };

        let model = TextEmbedding::try_new(
            TextInitOptions::new(parsed_model)
                .with_cache_dir(PathBuf::from(cache_dir))
                .with_show_download_progress(false),
        )?;

        Ok(Self { model: Arc::new(StdMutex::new(model)) })
    }
}

#[async_trait]
impl Embedder for FastembedEmbedder {
    fn dimension(&self) -> usize {
        DIM
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError> {
        let model = Arc::clone(&self.model);
        let text = text.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut m = model.lock().map_err(|e| DomainError::Internal(format!("lock: {e}")))?;
            let mut out = m
                .embed(vec![text.as_str()], None)
                .map_err(|e| DomainError::Internal(format!("fastembed: {e}")))?;
            out.pop().ok_or_else(|| DomainError::Internal("empty embedding result".to_owned()))
        })
        .await
        .map_err(|e| DomainError::Internal(format!("join: {e}")))?
    }
}

/// Deterministic stub. Used by every test that needs an embedder without
/// paying for MiniLM. Generates a 384-dim vector from a hash of the input.
#[derive(Debug, Default, Clone)]
pub struct StubEmbedder;

impl StubEmbedder {
    /// Construct an empty stub embedder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Embedder for StubEmbedder {
    fn dimension(&self) -> usize {
        DIM
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, DomainError> {
        // Cheap deterministic hash → 384 floats in [-1, 1].
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();
        let v: Vec<f32> = (0..DIM)
            .map(|i| {
                let x = seed.wrapping_add(i as u64) as f32 / u64::MAX as f32;
                x.mul_add(2.0, -1.0)
            })
            .collect();
        // L2-normalize so cosine similarity behaves predictably.
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 { Ok(v.into_iter().map(|x| x / norm).collect()) } else { Ok(v) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_embeds_to_384_dims() {
        let e = StubEmbedder::new();
        let v = e.embed("hello").await.unwrap();
        assert_eq!(v.len(), DIM);
    }

    #[tokio::test]
    async fn stub_is_deterministic() {
        let e = StubEmbedder::new();
        let a = e.embed("hello").await.unwrap();
        let b = e.embed("hello").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn stub_different_text_different_vector() {
        let e = StubEmbedder::new();
        let a = e.embed("hello").await.unwrap();
        let b = e.embed("world").await.unwrap();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn stub_is_l2_normalized() {
        let e = StubEmbedder::new();
        let v = e.embed("anything").await.unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "norm was {norm}");
    }
}
