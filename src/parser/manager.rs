//! Block manager for efficient round-trip editing and re-parsing

use std::collections::HashSet;

use super::{BlockDetector, HybridBlock, ParserRegistry};

/// Manages a collection of hybrid blocks with efficient update tracking
pub struct BlockManager {
    blocks: Vec<HybridBlock>,
    detector: BlockDetector,
    parser_registry: ParserRegistry,
    /// Track which blocks have been modified
    dirty_blocks: HashSet<usize>,
}

impl BlockManager {
    /// Create a new block manager
    #[must_use] 
    pub fn new(detector: BlockDetector, parser_registry: ParserRegistry) -> Self {
        Self {
            blocks: Vec::new(),
            detector,
            parser_registry,
            dirty_blocks: HashSet::new(),
        }
    }

    /// Parse a complete document into hybrid blocks
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable parser exists for a detected block's syntax
    pub fn parse_document(&mut self, text: &str) -> Result<(), String> {
        let raw_blocks = self.detector.detect(text);
        self.blocks.clear();
        self.dirty_blocks.clear();

        for raw_block in raw_blocks {
            let parser = self
                .parser_registry
                .get(raw_block.syntax)
                .ok_or_else(|| format!("No parser for {:?}", raw_block.syntax))?;

            let (ast, metadata) = parser
                .parse(&raw_block.content, raw_block.start_line)
                .map_err(|e| e.to_string())?;

            let hybrid_block = HybridBlock::new(
                raw_block.syntax,
                raw_block.content,
                ast,
                (raw_block.start_line, raw_block.end_line),
            )
            .with_metadata(metadata);

            self.blocks.push(hybrid_block);
        }

        Ok(())
    }

    /// Get all blocks
    #[must_use] 
    pub fn blocks(&self) -> &[HybridBlock] {
        &self.blocks
    }

    /// Get a mutable reference to a block
    pub fn block_mut(&mut self, index: usize) -> Option<&mut HybridBlock> {
        if self.blocks.get(index).is_some() {
            self.dirty_blocks.insert(index);
        }
        self.blocks.get_mut(index)
    }

    /// Update a block's raw text and re-parse it
    ///
    /// # Errors
    ///
    /// Returns an error if parsing the new text fails
    pub fn update_block_text(&mut self, index: usize, new_text: String) -> Result<(), String> {
        let block = self
            .blocks
            .get_mut(index)
            .ok_or("Block index out of range")?;

        let parser = self
            .parser_registry
            .get(block.syntax)
            .ok_or_else(|| format!("No parser for {:?}", block.syntax))?;

        let (ast, metadata) = parser
            .parse(&new_text, block.line_range.0)
            .map_err(|e| e.to_string())?;

        block.raw_text = new_text;
        block.ast = ast;
        block.metadata = metadata;
        self.dirty_blocks.insert(index);

        Ok(())
    }

    /// Get blocks that have been modified since last clear
    #[must_use] 
    pub fn dirty_blocks(&self) -> Vec<usize> {
        self.dirty_blocks.iter().copied().collect()
    }

    /// Clear the dirty block tracking
    pub fn clear_dirty(&mut self) {
        self.dirty_blocks.clear();
    }

    /// Insert a new block at the given index
    pub fn insert_block(&mut self, index: usize, block: HybridBlock) {
        self.blocks.insert(index, block);
        self.dirty_blocks.insert(index);
        for i in (index + 1)..self.blocks.len() {
            self.dirty_blocks.insert(i);
        }
    }

    /// Remove a block at the given index
    pub fn remove_block(&mut self, index: usize) -> Option<HybridBlock> {
        if index < self.blocks.len() {
            let removed = self.blocks.remove(index);
            self.dirty_blocks.remove(&index);
            for i in index..self.blocks.len() {
                self.dirty_blocks.insert(i);
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Render all blocks back to a complete document
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable parser exists for rendering a block's syntax
    pub fn render_document(&self) -> Result<String, String> {
        let mut output = String::new();

        for block in &self.blocks {
            let parser = self
                .parser_registry
                .get(block.syntax)
                .ok_or_else(|| format!("No parser for {:?}", block.syntax))?;

            let rendered = parser.render(&block.ast, &block.metadata);
            output.push_str(&rendered);
            output.push('\n');
        }

        Ok(output.trim_end().to_string())
    }

    /// Render only dirty blocks
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable parser exists for rendering a block's syntax
    pub fn render_dirty_blocks(&self) -> Result<String, String> {
        let mut output = String::new();

        for index in self.dirty_blocks() {
            let block = &self.blocks[index];
            let parser = self
                .parser_registry
                .get(block.syntax)
                .ok_or_else(|| format!("No parser for {:?}", block.syntax))?;

            let rendered = parser.render(&block.ast, &block.metadata);
            output.push_str(&rendered);
            output.push('\n');
        }

        Ok(output)
    }

    /// Get total number of blocks
    #[must_use] 
    pub const fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Find blocks by heading level
    #[must_use] 
    pub fn find_blocks_by_heading_level(&self, level: u8) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.metadata.heading_level == Some(level))
            .map(|(i, _)| i)
            .collect()
    }

    /// Find all heading blocks
    #[must_use] 
    pub fn find_headings(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.is_heading())
            .map(|(i, _)| i)
            .collect()
    }
}

impl Default for BlockManager {
    fn default() -> Self {
        Self::new(BlockDetector::default(), ParserRegistry::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::Block;
    use crate::parser::SyntaxKind;

    use super::*;

    #[test]
    fn test_block_manager_initialization() {
        let manager = BlockManager::default();
        assert_eq!(manager.block_count(), 0);
        assert!(manager.dirty_blocks().is_empty());
    }

    #[test]
    fn test_dirty_block_tracking() {
        let mut manager = BlockManager::default();
        let block = HybridBlock::new(
            SyntaxKind::Markdown,
            "test".to_string(),
            Block::paragraph(vec![]),
            (0, 0),
        );
        manager.insert_block(0, block);

        assert!(!manager.dirty_blocks().is_empty());
        manager.clear_dirty();
        assert!(manager.dirty_blocks().is_empty());
    }
}
