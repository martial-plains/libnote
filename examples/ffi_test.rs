//! Example showing how to use the FFI interface (LibnoteDocument)
//!
//! This demonstrates the cross-platform interface that would be used
//! from Swift, Kotlin, Python, etc.

use note::models::{Block, Note};

fn main() {
    println!("=== LibnoteDocument FFI Example ===\n");

    // Create a new note
    let mut note = Note {
        id: "doc-1".to_string(),
        title: "Document".to_string(),
        blocks: Vec::new(),
    };
    println!("✓ Created new LibnoteDocument");

    // Add some blocks to the note
    use note::models::{Inline, LeafBlock};

    note.blocks.push(Block::Leaf {
        leaf: LeafBlock::Heading {
            level: 1,
            content: vec![Inline::Text {
                text: "Project Plan".to_string(),
            }],
        },
    });

    note.blocks.push(Block::Leaf {
        leaf: LeafBlock::Paragraph {
            content: vec![Inline::Text {
                text: "This is a note with structured content.".to_string(),
            }],
        },
    });

    // Display blocks
    println!("\n✓ Note contains {} blocks\n", note.blocks.len());

    for (i, block) in note.blocks.iter().enumerate() {
        println!("Block {}: {:?}", i, block);
    }

    println!("\n=== FFI Example Complete ===");
    println!("\nThis interface is accessible from:");
    println!("  • Swift (iOS/macOS)");
    println!("  • Kotlin (Android)");
    println!("  • Python");
    println!("  • C/C++");
    println!("  • Other FFI-compatible languages");
}
