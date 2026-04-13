use crate::domain::{Embedder, Embedding, RascalError};

/// Stub embedder — returns a zero vector of fixed dimension.
/// All similarity scores will be 0.0 until the real adapter is implemented.
/// See: https://github.com/89jobrien/rascal/issues/1
pub struct StubEmbedder {
    pub dim: usize,
}

impl Default for StubEmbedder {
    fn default() -> Self {
        Self { dim: 1536 }
    }
}

impl Embedder for StubEmbedder {
    fn embed(&self, _text: &str) -> Result<Embedding, RascalError> {
        eprintln!(
            "rascal: [warn] StubEmbedder active — scores are meaningless (issue #1)"
        );
        Ok(vec![0.0_f32; self.dim])
    }
}
