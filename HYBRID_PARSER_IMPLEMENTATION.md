# Hybrid Parser Framework - Implementation Summary

## Overview
A comprehensive hybrid parser framework has been implemented for libnote that enables documents to contain multiple markup or code syntaxes (Markdown, Org-mode, LaTeX, custom markup, code blocks) while preserving round-trip fidelity and supporting efficient incremental re-parsing.

## Architecture

### Core Components

#### 1. **Block Detection** (`src/parser/detector.rs`)
- `BlockDetector`: Scans documents line-by-line to identify block boundaries
- `SyntaxBlock`: Raw block representation with syntax type and line ranges
- Supports detection of:
  - Markdown blocks (regular text until empty line or marker)
  - Org-mode blocks (`#+BEGIN_...#+END_...`)
  - Markdown code fences (```language...```)
  - LaTeX math blocks (`$$...$$` and `\[...\]`)

#### 2. **Parser Interface** (`src/parser/interface.rs`)
- `Parser` trait: Pluggable interface for syntax-specific parsers
  - `parse()`: Convert raw text to AST + metadata
  - `render()`: Convert AST back to raw text
  - `can_handle()`: Determine if parser handles given syntax
- `ParserRegistry`: Manages available parsers by syntax kind
- `ParseError` and `ParseResult` types

#### 3. **Example Parsers** (`src/parser/parsers/`)
- `MarkdownParser`: Handles Markdown syntax
  - Detects headings (#, ##, ###, etc.)
  - Parses paragraphs and horizontal rules
  - Extracts heading metadata

- `OrgParser`: Handles Org-mode syntax
  - Detects headings (*, **, ***, etc.)
  - Extracts TODO/DONE states
  - Parses source code blocks

- `LaTeXParser`: Handles LaTeX math syntax
  - Processes display math (`$$...$$`)
  - Handles LaTeX notation (`\[...\]`)

#### 4. **Block Manager** (`src/parser/manager.rs`)
- `BlockManager`: Orchestrates block detection, parsing, and management
- Features:
  - Full document parsing
  - Efficient block modification tracking (dirty blocks)
  - Per-block re-parsing without affecting other blocks
  - Block insertion/removal with automatic line tracking
  - Search capabilities (find headings, TODO items, etc.)

#### 5. **Data Models** (`src/parser/mod.rs` and `src/models.rs`)
- `SyntaxKind`: Enum identifying block syntax (Markdown, Org, LaTeX, Code, Custom)
- `HybridBlock`: Complete block representation
  - Syntax type
  - Raw text (for round-trip fidelity)
  - Parsed AST
  - Metadata (heading level, TODO state, properties, ID)
  - Line range tracking
  - Helper methods for querying block properties

- `BlockMetadata`: Per-block metadata
  - Heading level
  - Block ID
  - TODO state
  - Custom properties key-value pairs

- `HybridNote`: Document with hybrid blocks
  - Block management methods
  - Heading and TODO item queries
  - Block insertion/removal/access

- `Document`: Unified document representation (enum)
  - Can be either Standard (AST-based) or Hybrid (mixed syntax)
  - Format detection and conversion methods

## Key Features

### 1. **Block-Level Splitting**
- ✅ Scans document line-by-line
- ✅ Identifies blocks by syntax markers
- ✅ Preserves raw text for round-trip fidelity

### 2. **Mini-AST Parsing Per Block**
- ✅ Each block parsed with appropriate parser
- ✅ Produces AST capturing structure (headings, lists, etc.)
- ✅ Pluggable parser architecture for new syntaxes

### 3. **Unified Document Representation**
- ✅ Blocks stored in sequence with type and AST
- ✅ Per-block metadata (heading level, IDs, TODO states)
- ✅ Rendering, editing, and exporting without losing structure

### 4. **Round-Trip Editing**
- ✅ Modifications within blocks don't break other blocks
- ✅ Efficient re-parsing of only modified blocks
- ✅ Dirty block tracking for performance

## Testing

All 15 parser-specific tests pass:
- Block detection tests (Markdown, Org, LaTeX, Code)
- Parser tests (Markdown, Org, LaTeX)
- Block manager tests
- All metadata and metadata extraction

## Example Usage

See `examples/hybrid_parser_demo.rs` for a complete demonstration:

```rust
// 1. Create detector and registry
let detector = BlockDetector::new();
let mut registry = ParserRegistry::new();
registry.register(Box::new(MarkdownParser));
registry.register(Box::new(OrgParser));
registry.register(Box::new(LaTeXParser));

// 2. Parse document
let mut manager = BlockManager::new(detector, registry);
manager.parse_document(mixed_markup_text)?;

// 3. Query blocks
for block in manager.blocks() {
    if block.is_heading() {
        println!("Found heading level {}", block.heading_level().unwrap());
    }
}

// 4. Create hybrid document
let mut doc = Document::hybrid("id".into(), "Title".into());
if let Some(hybrid) = doc.as_hybrid_mut() {
    for block in manager.blocks() {
        hybrid.add_block(block.clone());
    }
}
```

## Extensibility

### Adding New Parsers
1. Implement the `Parser` trait:
   ```rust
   pub struct CustomParser;
   
   impl Parser for CustomParser {
       fn syntax_kind(&self) -> SyntaxKind { SyntaxKind::Custom("custom".into()) }
       fn parse(&self, raw_text: &str, line_offset: usize) -> ParseResult<(Block, BlockMetadata)> { ... }
       fn render(&self, block: &Block, metadata: &BlockMetadata) -> String { ... }
       fn can_handle(&self, text: &str) -> bool { ... }
   }
   ```

2. Register with the parser registry:
   ```rust
   registry.register(Box::new(CustomParser));
   ```

### Adding New Block Markers
Update `BlockDetector::scan_block()` to recognize new syntax markers and call appropriate scanning methods.

## Performance Considerations

- **Block-level granularity**: Only modified blocks are re-parsed
- **Dirty tracking**: Maintains set of modified block indices
- **Raw text preservation**: Enables efficient round-trip without re-rendering
- **Lazy evaluation**: Blocks parsed only when explicitly requested

## Future Enhancements

Potential areas for extension:
1. Rendering to multiple formats (HTML, PDF, etc.)
2. Document diffing and change tracking
3. Block-level versioning history
4. Advanced metadata querying (XPath-like expressions)
5. Custom block types and handlers
6. Performance optimization for large documents
7. Concurrent parsing for multi-block documents
