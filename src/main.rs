mod chunker;
mod config;
mod corpus;
mod domain;
mod embedder;
mod output;
mod scorer;

use std::path::PathBuf;
use std::process::ExitCode;

use chunker::TreeSitterChunker;
use config::Config;
use corpus::SqliteCorpusStore;
use domain::{ChunkKind, Chunker as _, CorpusStore as _, RascalError};
use embedder::StubEmbedder;
use scorer::Scorer;

// ============================================================================
// CLI
// ============================================================================

enum Command {
    Check { path: PathBuf, json: bool },
    Index { source: IndexSource },
    CorpusList,
    CorpusClear,
}

enum IndexSource {
    File(PathBuf),
    Dir(PathBuf),
}

fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [cmd, path] if cmd == "check" => Ok(Command::Check {
            path: PathBuf::from(path),
            json: false,
        }),
        [cmd, path, flag] if cmd == "check" && flag == "--json" => Ok(Command::Check {
            path: PathBuf::from(path),
            json: true,
        }),
        [cmd, path] if cmd == "index" => Ok(Command::Index {
            source: IndexSource::File(PathBuf::from(path)),
        }),
        [cmd, flag, dir] if cmd == "index" && flag == "--from" => Ok(Command::Index {
            source: IndexSource::Dir(PathBuf::from(dir)),
        }),
        [sub1, sub2] if sub1 == "corpus" && sub2 == "list" => Ok(Command::CorpusList),
        [sub1, sub2] if sub1 == "corpus" && sub2 == "clear" => Ok(Command::CorpusClear),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:
  rascal check <file> [--json]
  rascal index <file>
  rascal index --from <dir>
  rascal corpus list
  rascal corpus clear"
        .into()
}

// ============================================================================
// ENTRY POINT
// ============================================================================

fn main() -> ExitCode {
    match run() {
        Ok(passed) => {
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("rascal: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool, RascalError> {
    let cmd = parse_args().map_err(|e| {
        eprintln!("{e}");
        RascalError::Config("bad arguments".into())
    })?;

    let config = Config::load()?;
    let mut corpus = open_corpus()?;

    match cmd {
        Command::Check { path, json } => {
            let chunker = TreeSitterChunker {
                kinds: config.check_kinds.clone(),
            };
            let embedder = StubEmbedder::default();
            let scorer = Scorer {
                embedder: &embedder,
                corpus: &corpus,
                threshold: config.threshold,
            };

            let chunks = chunker.chunk(&path)?;
            if chunks.is_empty() {
                eprintln!("rascal: no chunks found in {}", path.display());
                return Ok(true);
            }

            let results = scorer.score_chunks(&chunks)?;
            let passed = results.iter().all(|r| r.passes);
            let path_str = path.display().to_string();

            if json {
                output::print_json(&results, &path_str, config.threshold);
            } else {
                output::print_table(&results, &path_str);
            }

            Ok(passed)
        }

        Command::Index { source } => {
            let chunker = TreeSitterChunker {
                kinds: vec![
                    ChunkKind::Function,
                    ChunkKind::ImplBlock,
                    ChunkKind::Trait,
                    ChunkKind::Struct,
                    ChunkKind::Enum,
                ],
            };

            let files = match source {
                IndexSource::File(p) => vec![p],
                IndexSource::Dir(d) => collect_rs_files(&d),
            };

            let mut total = 0usize;
            for file in &files {
                let chunks = chunker.chunk(file)?;
                for chunk in &chunks {
                    corpus.insert_entry(chunk, &[])?;
                    total += 1;
                }
            }

            println!("rascal: indexed {total} chunks from {} file(s)", files.len());
            Ok(true)
        }

        Command::CorpusList => {
            let entries = corpus.all_entries()?;
            if entries.is_empty() {
                println!("rascal: corpus is empty");
            } else {
                for e in &entries {
                    println!(
                        "{:>4}  {:12}  {}  ({})",
                        e.id,
                        e.kind.to_string(),
                        e.name,
                        e.source_path.display()
                    );
                }
                println!("rascal: {} entries", entries.len());
            }
            Ok(true)
        }

        Command::CorpusClear => {
            corpus.clear()?;
            println!("rascal: corpus cleared");
            Ok(true)
        }
    }
}

fn open_corpus() -> Result<SqliteCorpusStore, RascalError> {
    let path = dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("rascal")
        .join("corpus.db");
    SqliteCorpusStore::open(&path)
}

fn collect_rs_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_rs_recursive(dir, &mut out);
    out
}

fn collect_rs_recursive(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !matches!(name, "target" | ".git" | "node_modules") {
                collect_rs_recursive(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
