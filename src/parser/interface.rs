//! Parser interface and traits for extensibility

use crate::models::Block;
use std::{collections::HashMap, fmt::Debug};

use super::{BlockMetadata, SyntaxKind};

// Re-export error types for convenience
pub use crate::error::{ParseError, ParseResult};

/// Trait for implementing parsers for different syntaxes
pub trait Parser: Send + Sync + Debug {
    /// Get the syntax kind this parser handles
    fn syntax_kind(&self) -> SyntaxKind;

    /// Parse raw text into an AST block and extract metadata
    ///
    /// # Errors
    ///
    /// Returns a parsing error if the text cannot be properly parsed
    fn parse(&self, raw_text: &str, line_offset: usize) -> ParseResult<(Block, BlockMetadata)>;

    /// Render a block back to raw text (for serialization)
    fn render(&self, block: &Block, metadata: &BlockMetadata) -> String;

    /// Check if this parser can handle the given raw text
    fn can_handle(&self, text: &str) -> bool;
}

/// Registry for managing available parsers
pub struct ParserRegistry {
    parsers: HashMap<SyntaxKind, Box<dyn Parser>>,
}

impl ParserRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    /// Register a parser for a syntax kind
    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.insert(parser.syntax_kind(), parser);
    }

    /// Get a parser for the given syntax kind
    #[must_use]
    pub fn get(&self, syntax: SyntaxKind) -> Option<&dyn Parser> {
        self.parsers.get(&syntax).map(std::convert::AsRef::as_ref)
    }

    /// List all registered syntax kinds
    #[must_use]
    pub fn available_syntaxes(&self) -> Vec<SyntaxKind> {
        self.parsers.keys().copied().collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
