use std::path::Path;

use rascal::chunker::TreeSitterChunker;
use rascal::domain::{ChunkKind, Chunker};

fn all_kinds() -> Vec<ChunkKind> {
    vec![
        ChunkKind::Function,
        ChunkKind::ImplBlock,
        ChunkKind::Trait,
        ChunkKind::Struct,
        ChunkKind::Enum,
    ]
}

fn fixture_path() -> &'static Path {
    Path::new("tests/fixtures/sample.rs")
}

#[test]
fn extracts_expected_chunk_count() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    // greet (fn), MyError (struct), impl Display for MyError (impl) +
    // fmt (fn inside impl), Greeter (trait), Status (enum) = 6
    assert_eq!(chunks.len(), 6, "got: {:?}", chunks.iter().map(|c| &c.name).collect::<Vec<_>>());
}

#[test]
fn extracts_function_by_name() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(chunks.iter().any(|c| c.name == "greet" && c.kind == ChunkKind::Function));
}

#[test]
fn extracts_struct_by_name() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(chunks.iter().any(|c| c.name == "MyError" && c.kind == ChunkKind::Struct));
}

#[test]
fn extracts_impl_block_with_synthesized_name() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(
        chunks.iter().any(|c| c.kind == ChunkKind::ImplBlock && c.name.contains("Display")),
        "impl names: {:?}", chunks.iter().filter(|c| c.kind == ChunkKind::ImplBlock).map(|c| &c.name).collect::<Vec<_>>()
    );
}

#[test]
fn extracts_trait_by_name() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(chunks.iter().any(|c| c.name == "Greeter" && c.kind == ChunkKind::Trait));
}

#[test]
fn extracts_enum_by_name() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(chunks.iter().any(|c| c.name == "Status" && c.kind == ChunkKind::Enum));
}

#[test]
fn snippets_are_non_empty_substrings_of_source() {
    let chunker = TreeSitterChunker { kinds: all_kinds() };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    let source = std::fs::read_to_string(fixture_path()).unwrap();
    for chunk in &chunks {
        assert!(!chunk.snippet.is_empty(), "empty snippet for {}", chunk.name);
        assert!(source.contains(&chunk.snippet), "snippet not in source for {}", chunk.name);
    }
}

#[test]
fn function_only_filter_excludes_structs_and_traits() {
    let chunker = TreeSitterChunker {
        kinds: vec![ChunkKind::Function],
    };
    let chunks = chunker.chunk(fixture_path()).unwrap();
    assert!(chunks.iter().all(|c| c.kind == ChunkKind::Function));
}
