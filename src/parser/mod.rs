//! Hybrid parser framework supporting multiple markup syntaxes
//!
//! This module provides a framework for parsing documents that may contain
//! multiple markup languages (Markdown, Org-mode, LaTeX, custom, etc.).
//!
//! Key design principles:
//! - Block-level splitting by syntax markers
//! - Pluggable parsers for each syntax
//! - Round-trip fidelity with raw text preservation
//! - Efficient re-parsing of only modified blocks

pub mod detector;
pub mod interface;
pub mod manager;
pub mod parsers;

pub use detector::{BlockDetector, SyntaxBlock};
pub use interface::{Parser, ParserRegistry};
pub use manager::BlockManager;
pub use parsers::{LaTeXParser, MarkdownParser, OrgParser};

use serde::{Deserialize, Serialize};

/// Syntax type identifier for blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyntaxKind {
    /// Markdown syntax
    Markdown,
    /// Org-mode syntax
    Org,
    /// LaTeX / Math syntax
    LaTeX,
    /// Generic code block
    Code,
    /// Custom or unknown syntax
    Custom,
}

impl SyntaxKind {
    /// Get a human-readable name for this syntax
    #[must_use] 
    pub const fn name(&self) -> &str {
        match self {
            Self::Markdown => "Markdown",
            Self::Org => "Org-mode",
            Self::LaTeX => "LaTeX",
            Self::Code => "Code",
            Self::Custom => "Custom",
        }
    }
}

/// Block metadata for tracking structure and state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub struct BlockMetadata {
    /// Heading level (if this is a heading block)
    pub heading_level: Option<u8>,
    /// Block ID for referencing
    pub id: Option<String>,
    /// TODO state (Org-mode specific)
    pub todo_state: Option<String>,
    /// Custom properties/tags
    pub properties: Vec<(String, String)>,
}


/// A block with its syntax type, raw text, and parsed AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HybridBlock {
    /// The syntax type of this block
    pub syntax: SyntaxKind,
    /// Raw source text (for round-trip fidelity)
    pub raw_text: String,
    /// Parsed AST representation
    pub ast: crate::models::Block,
    /// Metadata extracted during parsing
    pub metadata: BlockMetadata,
    /// Line range in original document (start, end inclusive)
    pub line_range: (usize, usize),
}

impl HybridBlock {
    /// Create a new hybrid block
    #[must_use] 
    pub fn new(
        syntax: SyntaxKind,
        raw_text: String,
        ast: crate::models::Block,
        line_range: (usize, usize),
    ) -> Self {
        Self {
            syntax,
            raw_text,
            ast,
            metadata: BlockMetadata::default(),
            line_range,
        }
    }

    /// Update metadata
    #[must_use] 
    pub fn with_metadata(mut self, metadata: BlockMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if this block has a heading
    #[must_use] 
    pub const fn is_heading(&self) -> bool {
        self.metadata.heading_level.is_some()
    }

    /// Get the heading level if this is a heading block
    #[must_use] 
    pub const fn heading_level(&self) -> Option<u8> {
        self.metadata.heading_level
    }

    /// Check if this is a TODO block
    #[must_use] 
    pub fn is_todo(&self) -> bool {
        matches!(self.metadata.todo_state, Some(ref state) if state == "TODO")
    }

    /// Check if this is a DONE block
    #[must_use] 
    pub fn is_done(&self) -> bool {
        matches!(self.metadata.todo_state, Some(ref state) if state == "DONE")
    }

    /// Get TODO state if present
    #[must_use] 
    pub fn todo_state(&self) -> Option<&str> {
        self.metadata.todo_state.as_deref()
    }

    /// Get the block ID if present
    #[must_use] 
    pub fn id(&self) -> Option<&str> {
        self.metadata.id.as_deref()
    }

    /// Set the block ID
    #[must_use] 
    pub fn with_id(mut self, id: String) -> Self {
        self.metadata.id = Some(id);
        self
    }

    /// Get line count of this block
    #[must_use] 
    pub const fn line_count(&self) -> usize {
        self.line_range.1 - self.line_range.0 + 1
    }

    /// Check if this block is of a specific syntax
    #[must_use] 
    pub fn is_syntax(&self, syntax: SyntaxKind) -> bool {
        self.syntax == syntax
    }

    /// Get all properties for this block
    #[must_use] 
    pub fn properties(&self) -> &[(String, String)] {
        &self.metadata.properties
    }

    /// Add a property to this block
    pub fn add_property(&mut self, key: String, value: String) {
        self.metadata.properties.push((key, value));
    }

    /// Get a property value by key
    #[must_use] 
    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.metadata
            .properties
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Check if this block has any properties
    #[must_use] 
    pub const fn has_properties(&self) -> bool {
        !self.metadata.properties.is_empty()
    }
}
