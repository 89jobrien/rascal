use comfy_table::{Attribute, Cell, Color, Table};

use crate::domain::ChunkResult;

pub fn print_table(results: &[ChunkResult], path: &str) {
    let mut table = Table::new();
    table.set_header(vec!["File", "Chunk", "Kind", "Score", "Nearest Match", "Pass"]);

    for r in results {
        let pass_cell = if r.passes {
            Cell::new("✓").fg(Color::Green).add_attribute(Attribute::Bold)
        } else {
            Cell::new("✗").fg(Color::Red).add_attribute(Attribute::Bold)
        };

        let score_cell = {
            let s = format!("{:.2}", r.score);
            if r.passes {
                Cell::new(s).fg(Color::Green)
            } else {
                Cell::new(s).fg(Color::Red)
            }
        };

        table.add_row(vec![
            Cell::new(path),
            Cell::new(&r.chunk.name),
            Cell::new(r.chunk.kind.to_string()),
            score_cell,
            Cell::new(r.nearest_match.as_deref().unwrap_or("—")),
            pass_cell,
        ]);
    }

    println!("{table}");
}

#[derive(serde::Serialize)]
struct JsonOutput<'a> {
    passed: bool,
    threshold: f32,
    results: Vec<JsonChunk<'a>>,
}

#[derive(serde::Serialize)]
struct JsonChunk<'a> {
    file: &'a str,
    chunk: &'a str,
    kind: String,
    score: f32,
    nearest_match: Option<&'a str>,
    delta_hint: Option<&'a str>,
    passes: bool,
    snippet: &'a str,
}

pub fn print_json(results: &[ChunkResult], path: &str, threshold: f32) {
    let passed = results.iter().all(|r| r.passes);

    let chunks = results
        .iter()
        .map(|r| JsonChunk {
            file: path,
            chunk: &r.chunk.name,
            kind: r.chunk.kind.to_string(),
            score: r.score,
            nearest_match: r.nearest_match.as_deref(),
            delta_hint: r.delta_hint.as_deref(),
            passes: r.passes,
            snippet: &r.chunk.snippet,
        })
        .collect();

    let output = JsonOutput {
        passed,
        threshold,
        results: chunks,
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
