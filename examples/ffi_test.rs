//! Example showing how to use the FFI interface (LibnoteDocument)
//!
//! This demonstrates the cross-platform interface that would be used
//! from Swift, Kotlin, Python, etc.

use note::ffi::LibnoteDocument;

fn main() {
    println!("=== LibnoteDocument FFI Example ===\n");

    // Create a new document
    let mut doc = LibnoteDocument::new();
    println!("✓ Created new LibnoteDocument");

    // Parse a mixed-syntax document
    let mixed_doc = r#"# Project Plan

This is a mixed-syntax document demonstrating the FFI interface.

## TODO Items

* TODO: Implement feature X
* TODO: Write tests
* DONE: Design architecture

Some Markdown content here.

$$E = mc^2$$

More content.
"#;

    match doc.parse(mixed_doc) {
        Ok(block_count) => {
            println!("✓ Parsed document into {} blocks\n", block_count);
        }
        Err(e) => {
            eprintln!("Error parsing: {}", e);
            return;
        }
    }

    // Get all blocks
    let blocks = doc.get_all_blocks();
    println!("Total blocks: {}\n", blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        println!(
            "Block {}: {} (lines {}-{})",
            i, block.syntax_type, block.start_line, block.end_line
        );
        if let Some(level) = block.heading_level {
            println!("  └─ Heading Level: {}", level);
        }
        if let Some(todo) = &block.todo_state {
            println!("  └─ TODO State: {}", todo);
        }
    }

    println!("\n✓ Heading blocks:");
    let headings = doc.find_headings();
    for idx in headings {
        if let Some(block) = doc.get_block(idx) {
            println!(
                "  Block {}: {} (Level {})",
                idx,
                block.syntax_type,
                block.heading_level.unwrap_or(0)
            );
        }
    }

    println!("\n✓ TODO items:");
    let todos = doc.find_todos();
    for idx in todos {
        if let Some(block) = doc.get_block(idx) {
            println!(
                "  Block {}: {} ({})",
                idx,
                block.syntax_type,
                block.todo_state.as_ref().unwrap_or(&"NO TODO".to_string())
            );
        }
    }

    println!("\n✓ DONE items:");
    let done = doc.find_done_items();
    for idx in done {
        if let Some(block) = doc.get_block(idx) {
            println!("  Block {}: {}", idx, block.syntax_type);
        }
    }

    // Render back to text
    match doc.render() {
        Ok(rendered) => {
            println!("\n✓ Rendered document ({} chars)", rendered.len());
            println!("Match: {}", rendered == mixed_doc);
        }
        Err(e) => {
            eprintln!("Error rendering: {}", e);
        }
    }

    println!("\n=== FFI Example Complete ===");
    println!("\nThis interface is accessible from:");
    println!("  • Swift (iOS/macOS)");
    println!("  • Kotlin (Android)");
    println!("  • Python");
    println!("  • C/C++");
    println!("  • Other FFI-compatible languages");
}
