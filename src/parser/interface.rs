//! Parser interface and traits for extensibility

use crate::models::Block;
use std::collections::HashMap;

use super::{BlockMetadata, SyntaxKind};

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Error types for parsing
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Syntax error at line with message
    SyntaxError { line: usize, message: String },
    /// Unsupported syntax type
    UnsupportedSyntax(String),
    /// Block detection failed
    DetectionFailed(String),
    /// Other parsing error
    Other(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SyntaxError { line, message } => {
                write!(f, "Syntax error at line {line}: {message}")
            }
            Self::UnsupportedSyntax(s) => write!(f, "Unsupported syntax: {s}"),
            Self::DetectionFailed(s) => write!(f, "Block detection failed: {s}"),
            Self::Other(s) => write!(f, "Parse error: {s}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Trait for implementing parsers for different syntaxes
pub trait Parser: Send + Sync {
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
