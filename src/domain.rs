use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// VALUE TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkKind {
    Function,
    ImplBlock,
    Trait,
    Struct,
    Enum,
}

impl std::fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkKind::Function => write!(f, "function"),
            ChunkKind::ImplBlock => write!(f, "impl_block"),
            ChunkKind::Trait => write!(f, "trait"),
            ChunkKind::Struct => write!(f, "struct"),
            ChunkKind::Enum => write!(f, "enum"),
        }
    }
}

/// A semantic unit extracted from a source file.
#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub name: String,
    pub kind: ChunkKind,
    pub source_path: PathBuf,
    pub byte_range: (usize, usize),
    pub snippet: String,
}

/// A dense embedding vector.
pub type Embedding = Vec<f32>;

/// A row stored in the corpus.
#[derive(Debug, Clone, Serialize)]
pub struct CorpusEntry {
    pub id: i64,
    pub name: String,
    pub kind: ChunkKind,
    pub tags: Vec<String>,
    pub source_path: PathBuf,
    pub snippet: String,
}

/// The result of scoring one chunk against the corpus.
#[derive(Debug, Serialize)]
pub struct ChunkResult {
    pub chunk: Chunk,
    /// Cosine similarity in [0, 1]. 0.0 when stub embedder is active.
    pub score: f32,
    /// Name of the closest corpus entry, if any.
    pub nearest_match: Option<String>,
    /// Future: structural diff hint. None until real embedder lands.
    pub delta_hint: Option<String>,
    /// True when score >= threshold OR corpus is empty.
    pub passes: bool,
}

// ============================================================================
// ERRORS
// ============================================================================

#[derive(Debug, Error)]
pub enum RascalError {
    #[error("config error: {0}")]
    Config(String),

    #[error("parse error in {path}: {msg}")]
    Parse { path: String, msg: String },

    #[error("corpus error: {0}")]
    Corpus(String),

    #[error("embedder error: {0}")]
    Embedder(String),

    #[error("no chunks found in {0}")]
    NoChunks(String),
}

// ============================================================================
// PORTS (TRAITS)
// ============================================================================

/// Splits a source file into semantic chunks.
pub trait Chunker {
    fn chunk(&self, path: &Path) -> Result<Vec<Chunk>, RascalError>;
}

/// Produces an embedding vector for a text snippet.
/// Deferred — only StubEmbedder is shipped now (GitHub issue #1).
pub trait Embedder {
    fn embed(&self, text: &str) -> Result<Embedding, RascalError>;
}

/// Persistent store for corpus entries and their embeddings.
pub trait CorpusStore {
    fn insert_entry(&mut self, chunk: &Chunk, tags: &[String]) -> Result<i64, RascalError>;
    fn update_embedding(&mut self, corpus_id: i64, embedding: &Embedding)
        -> Result<(), RascalError>;
    fn all_entries(&self) -> Result<Vec<CorpusEntry>, RascalError>;
    fn all_embeddings(&self) -> Result<Vec<(i64, Embedding)>, RascalError>;
    fn clear(&mut self) -> Result<(), RascalError>;
}

// ============================================================================
// PURE DOMAIN FUNCTIONS
// ============================================================================

/// Returns cosine similarity in [0, 1].
/// Returns 0.0 if either vector is zero-magnitude or slices are empty.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "embedding dimension mismatch");
    if a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_score_one() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn orthogonal_vectors_score_zero() {
        assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0])).abs() < f32::EPSILON);
    }

    #[test]
    fn zero_vector_scores_zero() {
        assert!((cosine_similarity(&[0.0, 0.0], &[1.0, 1.0])).abs() < f32::EPSILON);
    }

    #[test]
    fn known_angle_45_degrees() {
        // [1,1] and [1,0] → cos(45°) ≈ 0.707
        let score = cosine_similarity(&[1.0, 1.0], &[1.0, 0.0]);
        assert!((score - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn identical_unit_vectors_score_one() {
        let v = vec![0.6, 0.8];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }
}
