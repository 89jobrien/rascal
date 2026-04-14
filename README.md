# rascal

Semantic code linter for Rust. Parses `.rs` files into chunks via tree-sitter, scores each chunk against a corpus of golden idiomatic examples using cosine similarity, and flags anything below the configured threshold.

> Note: `StubEmbedder` is active until issue #1 (OpenAI-compatible embedder) lands — all scores are currently 0.0 / passing.

## Install

```bash
cargo install --path .
```

## Usage

```bash
rascal check <file>           # score all chunks in a file
rascal check <file> --json    # machine-readable output
rascal index <file>           # add a file to the corpus
rascal index --from <dir>     # add all .rs files in a directory tree
rascal corpus list            # list corpus entries
rascal corpus clear           # remove all corpus entries
```

`check` exits 0 if all chunks pass, 1 if any fail.

## Configuration

`~/.config/rascal/config.toml` — all fields are optional.

```toml
api_url   = "https://api.openai.com/v1"
api_key   = "sk-..."
model     = "text-embedding-3-small"
threshold = 0.75   # cosine similarity cutoff (default: 0.75)

# chunk kinds scored during check (default: function, impl_block, trait)
check_kinds = ["function", "impl_block", "trait", "struct", "enum"]
```

The corpus is stored at `~/.local/share/rascal/corpus.db` (SQLite).

## License

MIT OR Apache-2.0
