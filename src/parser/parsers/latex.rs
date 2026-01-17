//! LaTeX/Math parser implementation
#![allow(clippy::unused_self)]

use crate::models::Block;
use crate::parser::{BlockMetadata, SyntaxKind};

use super::super::interface::{ParseResult, Parser};

/// Parser for LaTeX math syntax
pub struct LaTeXParser;

impl Parser for LaTeXParser {
    fn syntax_kind(&self) -> SyntaxKind {
        SyntaxKind::LaTeX
    }

    fn parse(&self, raw_text: &str, _line_offset: usize) -> ParseResult<(Block, BlockMetadata)> {
        let metadata = BlockMetadata::default();

        let content = self.extract_math_content(raw_text);

        Ok((Block::math_block(content), metadata))
    }

    fn render(&self, block: &Block, _metadata: &BlockMetadata) -> String {
        match block {
            Block::Leaf {
                leaf: crate::models::LeafBlock::MathBlock { content },
            } => {
                format!("$${content}$$")
            }
            _ => String::new(),
        }
    }

    fn can_handle(&self, text: &str) -> bool {
        text.contains("$$") || text.contains("\\[") || text.contains("\\]")
    }
}

impl LaTeXParser {
    /// Extract math content from LaTeX delimiters
    fn extract_math_content(&self, text: &str) -> String {
        if text.contains("$$") {
            return text
                .replace("$$", "")
                .trim()
                .to_string();
        }

        if text.contains("\\[") {
            return text
                .replace("\\[", "")
                .replace("\\]", "")
                .trim()
                .to_string();
        }

        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latex_display_math() {
        let parser = LaTeXParser;
        let (block, _) = parser.parse("$$E = mc^2$$", 0).unwrap();

        assert!(matches!(
            block,
            Block::Leaf {
                leaf: crate::models::LeafBlock::MathBlock { .. }
            }
        ));
    }

    #[test]
    fn parse_latex_bracket_notation() {
        let parser = LaTeXParser;
        let (block, _) = parser.parse("\\[x^2 + y^2 = z^2\\]", 0).unwrap();

        assert!(matches!(
            block,
            Block::Leaf {
                leaf: crate::models::LeafBlock::MathBlock { .. }
            }
        ));
    }

    #[test]
    fn can_handle_latex() {
        let parser = LaTeXParser;
        assert!(parser.can_handle("$$math$$"));
        assert!(parser.can_handle("\\[math\\]"));
        assert!(!parser.can_handle("regular text"));
    }
}
