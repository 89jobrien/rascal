# rascal

Semantic code linter for Rust. 

## Features

- Parses .rs files into chunks via tree-sitter-rust
- Scores each chunk against a corpus of golden idiomatic examples using cosine similarity
- StubEmbedder active until issue #1 (OpenAI-compatible embedder) lands — all scores currently 0.0 / passing.
- SQLite corpus via rusqlite.
- CLI: check, index, corpus list/clear.
