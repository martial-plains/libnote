//! Demonstration of the hybrid parser framework
//!
//! This example shows how to:
//! 1. Detect blocks with mixed markup syntaxes
//! 2. Parse each block with appropriate parsers
//! 3. Manage hybrid documents with round-trip fidelity

use note::models::Document;
use note::parser::{BlockDetector, BlockManager, ParserRegistry};
use note::parser::{LaTeXParser, MarkdownParser, OrgParser};

fn main() {
    println!("=== Hybrid Parser Framework Demo ===\n");

    // Example document with mixed markup
    let mixed_doc = r#"# Main Title

This is a regular Markdown paragraph.

#+BEGIN_SRC rust
fn hello_world() {
    println!("Hello from Rust!");
}
#+END_SRC

## Org-mode Section

* TODO Task 1
* DONE Task 2

Some more text here.

$$E = mc^2$$

Final paragraph with **bold** text."#;

    println!("Input Document:\n{}\n", mixed_doc);
    println!("---\n");

    // Step 1: Detect blocks
    let detector = BlockDetector::new();
    let raw_blocks = detector.detect(mixed_doc);

    println!("Detected {} blocks:\n", raw_blocks.len());
    for (i, block) in raw_blocks.iter().enumerate() {
        println!(
            "  Block {}: {:?} (lines {}-{})",
            i, block.syntax, block.start_line, block.end_line
        );
    }
    println!();

    // Step 2: Create parser registry and register parsers
    let mut registry = ParserRegistry::new();
    registry.register(Box::new(MarkdownParser));
    registry.register(Box::new(OrgParser));
    registry.register(Box::new(LaTeXParser));

    println!("Registered syntaxes:");
    for syntax in registry.available_syntaxes() {
        println!("  - {}", syntax.name());
    }
    println!();

    // Step 3: Create block manager and parse document
    let mut manager = BlockManager::new(detector, registry);
    match manager.parse_document(mixed_doc) {
        Ok(_) => {
            println!(
                "✓ Successfully parsed document into {} blocks\n",
                manager.block_count()
            );

            // Step 4: Analyze parsed blocks
            println!("Block Analysis:");
            for (i, block) in manager.blocks().iter().enumerate() {
                println!("  Block {}:", i);
                println!("    Syntax: {}", block.syntax.name());
                if let Some(level) = block.heading_level() {
                    println!("    Heading Level: {}", level);
                }
                if let Some(state) = block.todo_state() {
                    println!("    TODO State: {}", state);
                }
                println!("    Lines: {}", block.line_count());
            }
            println!();

            // Step 5: Find specific block types
            let headings = manager
                .blocks()
                .iter()
                .enumerate()
                .filter(|(_, b)| b.is_heading())
                .collect::<Vec<_>>();

            println!("Found {} heading blocks:", headings.len());
            for (i, block) in headings {
                if let Some(level) = block.heading_level() {
                    println!("  Block {}: Level {} heading", i, level);
                }
            }
            println!();

            // Step 6: Round-trip preservation
            println!("Raw text preservation check:");
            println!("  Original document length: {} chars", mixed_doc.len());
            println!(
                "  Total raw text in blocks: {} chars",
                manager
                    .blocks()
                    .iter()
                    .map(|b| b.raw_text.len())
                    .sum::<usize>()
            );
            println!();

            // Step 7: Demonstrate Document enum
            let mut doc = Document::hybrid(
                "demo-001".to_string(),
                "Hybrid Document Example".to_string(),
            );

            if let Some(hybrid) = doc.as_hybrid_mut() {
                for block in manager.blocks() {
                    hybrid.add_block(block.clone());
                }
            }

            println!("Created hybrid document:");
            println!("  ID: {}", doc.id());
            println!("  Title: {}", doc.title());
            println!("  Format: {:?}", doc.format());
            println!(
                "  Blocks: {}",
                doc.as_hybrid().map(|n| n.block_count()).unwrap_or(0)
            );
        }
        Err(e) => eprintln!("Failed to parse document: {}", e),
    }

    println!("\n=== Demo Complete ===");
}
