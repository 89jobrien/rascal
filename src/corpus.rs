use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::domain::{Chunk, ChunkKind, CorpusEntry, CorpusStore, Embedding, RascalError};

pub struct SqliteCorpusStore {
    conn: Connection,
}

impl SqliteCorpusStore {
    pub fn open(path: &Path) -> Result<Self, RascalError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RascalError::Corpus(format!("cannot create db dir: {e}")))?;
        }

        unsafe { sqlite_vec::sqlite3_vec_init(); }

        let conn = Connection::open(path)
            .map_err(|e| RascalError::Corpus(format!("cannot open db: {e}")))?;

        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    /// Open an in-memory database — useful for tests.
    pub fn in_memory() -> Result<Self, RascalError> {
        unsafe { sqlite_vec::sqlite3_vec_init(); }
        let conn = Connection::open_in_memory()
            .map_err(|e| RascalError::Corpus(format!("cannot open in-memory db: {e}")))?;
        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    fn ensure_schema(&self) -> Result<(), RascalError> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS corpus (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    name        TEXT    NOT NULL,
                    kind        TEXT    NOT NULL,
                    tags        TEXT    NOT NULL DEFAULT '[]',
                    source_path TEXT    NOT NULL,
                    snippet     TEXT    NOT NULL
                );

                CREATE TABLE IF NOT EXISTS embeddings (
                    corpus_id   INTEGER PRIMARY KEY
                                REFERENCES corpus(id) ON DELETE CASCADE,
                    embedding   BLOB    NOT NULL
                );
                ",
            )
            .map_err(|e| RascalError::Corpus(format!("schema error: {e}")))?;
        Ok(())
    }
}

impl CorpusStore for SqliteCorpusStore {
    fn insert_entry(&mut self, chunk: &Chunk, tags: &[String]) -> Result<i64, RascalError> {
        let tags_json = serde_json::to_string(tags)
            .map_err(|e| RascalError::Corpus(format!("tag serialization: {e}")))?;

        self.conn
            .execute(
                "INSERT INTO corpus (name, kind, tags, source_path, snippet)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    chunk.name,
                    chunk.kind.to_string(),
                    tags_json,
                    chunk.source_path.display().to_string(),
                    chunk.snippet,
                ],
            )
            .map_err(|e| RascalError::Corpus(format!("insert failed: {e}")))?;

        Ok(self.conn.last_insert_rowid())
    }

    fn update_embedding(
        &mut self,
        corpus_id: i64,
        embedding: &Embedding,
    ) -> Result<(), RascalError> {
        let blob = embedding_to_blob(embedding);
        self.conn
            .execute(
                "INSERT OR REPLACE INTO embeddings (corpus_id, embedding) VALUES (?1, ?2)",
                params![corpus_id, blob],
            )
            .map_err(|e| RascalError::Corpus(format!("embedding update failed: {e}")))?;
        Ok(())
    }

    fn all_entries(&self) -> Result<Vec<CorpusEntry>, RascalError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, kind, tags, source_path, snippet FROM corpus")
            .map_err(|e| RascalError::Corpus(e.to_string()))?;

        let entries = stmt
            .query_map([], |row| {
                let kind_str: String = row.get(2)?;
                let tags_json: String = row.get(3)?;
                let path_str: String = row.get(4)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    kind_str,
                    tags_json,
                    path_str,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| RascalError::Corpus(e.to_string()))?
            .filter_map(|r| r.ok())
            .map(|(id, name, kind_str, tags_json, path_str, snippet)| {
                let kind = parse_kind(&kind_str);
                let tags: Vec<String> =
                    serde_json::from_str(&tags_json).unwrap_or_default();
                CorpusEntry {
                    id,
                    name,
                    kind,
                    tags,
                    source_path: PathBuf::from(path_str),
                    snippet,
                }
            })
            .collect();

        Ok(entries)
    }

    fn all_embeddings(&self) -> Result<Vec<(i64, Embedding)>, RascalError> {
        let mut stmt = self
            .conn
            .prepare("SELECT corpus_id, embedding FROM embeddings")
            .map_err(|e| RascalError::Corpus(e.to_string()))?;

        let pairs = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob))
            })
            .map_err(|e| RascalError::Corpus(e.to_string()))?
            .filter_map(|r| r.ok())
            .map(|(id, blob)| (id, blob_to_embedding(&blob)))
            .collect();

        Ok(pairs)
    }

    fn clear(&mut self) -> Result<(), RascalError> {
        self.conn
            .execute_batch("DELETE FROM embeddings; DELETE FROM corpus;")
            .map_err(|e| RascalError::Corpus(format!("clear failed: {e}")))?;
        Ok(())
    }
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_embedding(blob: &[u8]) -> Embedding {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn parse_kind(s: &str) -> ChunkKind {
    match s {
        "function" => ChunkKind::Function,
        "impl_block" => ChunkKind::ImplBlock,
        "trait" => ChunkKind::Trait,
        "struct" => ChunkKind::Struct,
        _ => ChunkKind::Enum,
    }
}
