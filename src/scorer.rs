use crate::domain::{
    Chunk, ChunkResult, CorpusStore, Embedder, Embedding, RascalError, cosine_similarity,
};

pub struct Scorer<'a> {
    pub embedder: &'a dyn Embedder,
    pub corpus: &'a dyn CorpusStore,
    pub threshold: f32,
}

impl Scorer<'_> {
    pub fn score_chunks(&self, chunks: &[Chunk]) -> Result<Vec<ChunkResult>, RascalError> {
        let corpus_embeddings = self.corpus.all_embeddings()?;
        let corpus_entries = self.corpus.all_entries()?;
        let corpus_empty = corpus_embeddings.is_empty();

        chunks
            .iter()
            .map(|chunk| {
                let query = self.embedder.embed(&chunk.snippet)?;
                let (best_id, best_score) = nearest(&query, &corpus_embeddings);

                let nearest_match = best_id
                    .and_then(|id| corpus_entries.iter().find(|e| e.id == id))
                    .map(|e| e.name.clone());

                Ok(ChunkResult {
                    chunk: chunk.clone(),
                    score: best_score,
                    nearest_match,
                    delta_hint: None,
                    // Pass when corpus is empty (no false positives before indexing).
                    passes: corpus_empty || best_score >= self.threshold,
                })
            })
            .collect()
    }
}

fn nearest(query: &Embedding, corpus: &[(i64, Embedding)]) -> (Option<i64>, f32) {
    corpus
        .iter()
        .map(|(id, emb)| (*id, cosine_similarity(query, emb)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, score)| (Some(id), score))
        .unwrap_or((None, 0.0))
}
