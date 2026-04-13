use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::domain::{Chunk, ChunkKind, Chunker, RascalError};

pub struct TreeSitterChunker {
    pub kinds: Vec<ChunkKind>,
}

impl Chunker for TreeSitterChunker {
    fn chunk(&self, path: &Path) -> Result<Vec<Chunk>, RascalError> {
        let source = std::fs::read_to_string(path).map_err(|e| RascalError::Parse {
            path: path.display().to_string(),
            msg: e.to_string(),
        })?;

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| RascalError::Parse {
                path: path.display().to_string(),
                msg: e.to_string(),
            })?;

        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| RascalError::Parse {
                path: path.display().to_string(),
                msg: "tree-sitter returned no tree".into(),
            })?;

        let mut chunks = Vec::new();
        collect_chunks(
            tree.root_node(),
            source.as_bytes(),
            path,
            &self.kinds,
            &mut chunks,
        );

        Ok(chunks)
    }
}

/// Walk the top level of the tree (and one level into impl blocks) collecting
/// nodes whose kind matches `wanted`. Does not recurse into matched nodes.
fn collect_chunks(
    node: Node,
    source: &[u8],
    path: &Path,
    wanted: &[ChunkKind],
    out: &mut Vec<Chunk>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(kind) = node_chunk_kind(&child) {
            if wanted.contains(&kind) {
                if let Some(chunk) = build_chunk(child, source, path, kind) {
                    // For impl blocks, also collect inner functions if Function is wanted.
                    if chunk.kind == ChunkKind::ImplBlock && wanted.contains(&ChunkKind::Function) {
                        collect_impl_functions(child, source, path, out);
                    }
                    out.push(chunk);
                }
                // Do not recurse further into matched nodes.
                continue;
            }
        }
        // Recurse into non-matched top-level containers (e.g. mod items).
        collect_chunks(child, source, path, wanted, out);
    }
}

/// Collect function_item children of an impl_item's declaration_list.
fn collect_impl_functions(impl_node: Node, source: &[u8], path: &Path, out: &mut Vec<Chunk>) {
    let mut cursor = impl_node.walk();
    for child in impl_node.children(&mut cursor) {
        if child.kind() == "declaration_list" {
            let mut inner = child.walk();
            for item in child.children(&mut inner) {
                if item.kind() == "function_item" {
                    if let Some(chunk) = build_chunk(item, source, path, ChunkKind::Function) {
                        out.push(chunk);
                    }
                }
            }
        }
    }
}

fn node_chunk_kind(node: &Node) -> Option<ChunkKind> {
    match node.kind() {
        "function_item" => Some(ChunkKind::Function),
        "impl_item" => Some(ChunkKind::ImplBlock),
        "trait_item" => Some(ChunkKind::Trait),
        "struct_item" => Some(ChunkKind::Struct),
        "enum_item" => Some(ChunkKind::Enum),
        _ => None,
    }
}

fn build_chunk(node: Node, source: &[u8], path: &Path, kind: ChunkKind) -> Option<Chunk> {
    let snippet = node.utf8_text(source).ok()?.to_owned();
    let name = extract_name(&node, source, &kind);
    let byte_range = (node.start_byte(), node.end_byte());

    Some(Chunk {
        name,
        kind,
        source_path: path.to_path_buf(),
        byte_range,
        snippet,
    })
}

fn extract_name(node: &Node, source: &[u8], kind: &ChunkKind) -> String {
    match kind {
        ChunkKind::Function | ChunkKind::Trait | ChunkKind::Struct | ChunkKind::Enum => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("<unknown>")
            .to_owned(),

        ChunkKind::ImplBlock => {
            // Synthesize "impl Trait for Type" or "impl Type"
            let trait_node = node.child_by_field_name("trait");
            let type_node = node.child_by_field_name("type");

            let type_text = type_node
                .and_then(|n| n.utf8_text(source).ok())
                .unwrap_or("<unknown>");

            if let Some(trait_text) = trait_node.and_then(|n| n.utf8_text(source).ok()) {
                format!("impl {trait_text} for {type_text}")
            } else {
                format!("impl {type_text}")
            }
        }
    }
}
