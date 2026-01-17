//! `UniFFI` bindings for the hybrid parser framework
//!
//! This module provides a FFI interface to use the hybrid parser
//! from different platforms (iOS, Android, Python, etc.)
#![allow(
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc,
    clippy::unused_self
)]

use crate::parser::{BlockDetector, BlockManager, HybridBlock, ParserRegistry, SyntaxKind};
use crate::parser::{LaTeXParser, MarkdownParser, OrgParser};
use std::fmt;
use std::sync::Mutex;

/// A simplified block representation for FFI
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiBlock {
    pub syntax_type: String,
    pub raw_text: String,
    pub heading_level: Option<u8>,
    pub todo_state: Option<String>,
    pub block_id: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
}

impl FfiBlock {
    /// Create FFI block from `HybridBlock`
    fn from_hybrid(block: &HybridBlock) -> Self {
        Self {
            syntax_type: block.syntax.name().to_string(),
            raw_text: block.raw_text.clone(),
            heading_level: block.metadata.heading_level,
            todo_state: block.metadata.todo_state.clone(),
            block_id: block.metadata.id.clone(),
            start_line: block.line_range.0 as u32,
            end_line: block.line_range.1 as u32,
        }
    }
}

/// Error type for `LibnoteDocument` operations
#[derive(Debug, uniffi::Error)]
pub enum LibnoteError {
    ParseError(String),
    InvalidIndex,
    UnsupportedSyntax(String),
    Other(String),
}

impl fmt::Display for LibnoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(s) => write!(f, "Failed to parse document: {s}"),
            Self::InvalidIndex => write!(f, "Invalid block index"),
            Self::UnsupportedSyntax(s) => write!(f, "Unsupported syntax type: {s}"),
            Self::Other(s) => write!(f, "Operation failed: {s}"),
        }
    }
}

impl From<String> for LibnoteError {
    fn from(err: String) -> Self {
        Self::Other(err)
    }
}

/// Unified interface for working with documents across platforms
#[derive(uniffi::Object)]
pub struct LibnoteDocument {
    manager: Mutex<BlockManager>,
}

#[uniffi::export]
impl LibnoteDocument {
    /// Create a new empty Libnote document
    #[uniffi::constructor]
    #[must_use]
    pub fn new() -> Self {
        let detector = BlockDetector::new();
        let mut registry = ParserRegistry::new();

        registry.register(Box::new(MarkdownParser));
        registry.register(Box::new(OrgParser));
        registry.register(Box::new(LaTeXParser));

        let manager = BlockManager::new(detector, registry);

        Self {
            manager: Mutex::new(manager),
        }
    }

    /// Parse a document from raw text
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails
    ///
    /// # Panics
    ///
    /// May panic if the mutex is poisoned (this should not happen in normal operation)
    pub fn parse(&self, text: &str) -> Result<u32, LibnoteError> {
        let mut manager = self.manager.lock().unwrap();
        manager
            .parse_document(text)
            .map_err(LibnoteError::ParseError)?;
        Ok(manager.block_count() as u32)
    }

    /// Get total number of blocks
    pub fn block_count(&self) -> u32 {
        let manager = self.manager.lock().unwrap();
        manager.block_count() as u32
    }

    /// Get a block at the given index
    pub fn get_block(&self, index: u32) -> Option<FfiBlock> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .get(index as usize)
            .map(FfiBlock::from_hybrid)
    }

    /// Get all blocks
    pub fn get_all_blocks(&self) -> Vec<FfiBlock> {
        let manager = self.manager.lock().unwrap();
        manager.blocks().iter().map(FfiBlock::from_hybrid).collect()
    }

    /// Find all heading blocks
    pub fn find_headings(&self) -> Vec<u32> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.is_heading())
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Find headings at a specific level
    pub fn find_headings_at_level(&self, level: u8) -> Vec<u32> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.metadata.heading_level == Some(level))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Find all TODO items
    pub fn find_todos(&self) -> Vec<u32> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, block)| block.todo_state() == Some("TODO"))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Find all DONE items
    pub fn find_done_items(&self) -> Vec<u32> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, block)| block.todo_state() == Some("DONE"))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Update block raw text and re-parse
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails
    ///
    /// # Panics
    ///
    /// May panic if the mutex is poisoned
    pub fn update_block(&self, index: u32, new_text: String) -> Result<(), LibnoteError> {
        let mut manager = self.manager.lock().unwrap();
        manager
            .update_block_text(index as usize, new_text)
            .map_err(LibnoteError::ParseError)
    }

    /// Get dirty blocks (modified since last check)
    pub fn get_dirty_blocks(&self) -> Vec<u32> {
        let manager = self.manager.lock().unwrap();
        manager.dirty_blocks().iter().map(|i| *i as u32).collect()
    }

    /// Clear dirty block tracking
    pub fn clear_dirty(&self) {
        let mut manager = self.manager.lock().unwrap();
        manager.clear_dirty();
    }

    /// Render document back to text
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails
    ///
    /// # Panics
    ///
    /// May panic if the mutex is poisoned
    pub fn render(&self) -> Result<String, LibnoteError> {
        let manager = self.manager.lock().unwrap();
        manager.render_document().map_err(LibnoteError::Other)
    }

    /// Get syntax type name for a block
    pub fn get_block_syntax_name(&self, index: u32) -> Option<String> {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .get(index as usize)
            .map(|b| b.syntax.name().to_string())
    }

    /// Check if a block is a heading
    pub fn is_heading(&self, index: u32) -> bool {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .get(index as usize)
            .is_some_and(super::parser::HybridBlock::is_heading)
    }

    /// Check if a block is a TODO
    pub fn is_todo(&self, index: u32) -> bool {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .get(index as usize)
            .is_some_and(super::parser::HybridBlock::is_todo)
    }

    /// Check if a block is DONE
    pub fn is_done(&self, index: u32) -> bool {
        let manager = self.manager.lock().unwrap();
        manager
            .blocks()
            .get(index as usize)
            .is_some_and(super::parser::HybridBlock::is_done)
    }

    /// Insert a new block at the given index
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax type is not supported or parsing fails
    ///
    /// # Panics
    ///
    /// May panic if the mutex is poisoned
    pub fn insert_block(
        &self,
        index: u32,
        syntax: String,
        raw_text: String,
    ) -> Result<(), LibnoteError> {
        let syntax_kind = match syntax.to_lowercase().as_str() {
            "markdown" => SyntaxKind::Markdown,
            "org" => SyntaxKind::Org,
            "latex" => SyntaxKind::LaTeX,
            "code" => SyntaxKind::Code,
            _ => return Err(LibnoteError::UnsupportedSyntax(syntax)),
        };

        let registry = ParserRegistry::new();
        let parser = registry
            .get(syntax_kind)
            .ok_or(LibnoteError::UnsupportedSyntax(syntax))?;

        let (ast, metadata) = parser
            .parse(&raw_text, 0)
            .map_err(|e| LibnoteError::ParseError(e.to_string()))?;

        let block = HybridBlock::new(syntax_kind, raw_text, ast, (0, 0)).with_metadata(metadata);

        let mut manager = self.manager.lock().unwrap();
        manager.insert_block(index as usize, block);
        Ok(())
    }

    /// Remove a block at the given index
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds
    ///
    /// # Panics
    ///
    /// May panic if the mutex is poisoned
    pub fn remove_block(&self, index: u32) -> Result<(), LibnoteError> {
        let mut manager = self.manager.lock().unwrap();
        manager
            .remove_block(index as usize)
            .ok_or(LibnoteError::InvalidIndex)?;
        Ok(())
    }

    /// Render a block as `FfiBlock`
    pub fn render_block(&self, index: u32) -> Option<FfiBlock> {
        self.get_block(index)
    }
}

impl Default for LibnoteDocument {
    fn default() -> Self {
        Self::new()
    }
}
