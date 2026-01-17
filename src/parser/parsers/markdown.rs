//! Markdown parser implementation
#![allow(clippy::unused_self)]

use crate::models::{Block, Inline};
use crate::parser::{BlockMetadata, SyntaxKind};

use super::super::interface::{ParseResult, Parser};

/// Parser for Markdown syntax
#[derive(Debug)]
pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn syntax_kind(&self) -> SyntaxKind {
        SyntaxKind::Markdown
    }

    fn parse(&self, raw_text: &str, _line_offset: usize) -> ParseResult<(Block, BlockMetadata)> {
        let mut metadata = BlockMetadata::default();
        let lines: Vec<&str> = raw_text.lines().collect();

        if let Some(first_line) = lines.first()
            && let Some(level) = self.get_heading_level(first_line)
        {
            metadata.heading_level = Some(level);

            let heading_text = first_line.trim_start_matches('#').trim();
            let content = vec![Inline::Text {
                text: heading_text.to_string(),
            }];

            return Ok((Block::heading(level, content), metadata));
        }

        if lines.len() == 1 {
            let line = lines[0].trim();
            if (line.starts_with("---") || line.starts_with("***") || line.starts_with("___"))
                && line
                    .chars()
                    .all(|c| c == '-' || c == '*' || c == '_' || c.is_whitespace())
            {
                return Ok((Block::horizontal_rule(), metadata));
            }
        }

        let content = self.parse_inline_content(raw_text);
        Ok((Block::paragraph(content), metadata))
    }

    fn render(&self, block: &Block, _metadata: &BlockMetadata) -> String {
        match block {
            Block::Leaf {
                leaf: crate::models::LeafBlock::Heading { level, content },
            } => {
                let heading_marker = "#".repeat(*level as usize);
                let text = self.render_inline_content(content);
                format!("{heading_marker} {text}")
            }
            Block::Leaf {
                leaf: crate::models::LeafBlock::HorizontalRule,
            } => "---".to_string(),
            Block::Leaf {
                leaf: crate::models::LeafBlock::Paragraph { content },
            } => self.render_inline_content(content),
            Block::Leaf {
                leaf: crate::models::LeafBlock::CodeBlock { language, content },
            } => {
                let fence = if let Some(lang) = language {
                    format!("```{lang}")
                } else {
                    "```".to_string()
                };
                format!("{fence}\n{content}\n```")
            }
            _ => String::new(),
        }
    }

    fn can_handle(&self, text: &str) -> bool {
        !text.trim().is_empty() && !text.trim().starts_with("#+BEGIN_") && !text.contains("$$")
    }
}

impl MarkdownParser {
    /// Extract heading level from a line starting with #
    #[allow(clippy::cast_possible_truncation)]
    fn get_heading_level(&self, line: &str) -> Option<u8> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            return None;
        }

        let hash_count = trimmed.chars().take_while(|c| *c == '#').count();
        if hash_count > 0
            && hash_count < 7
            && (trimmed.get(hash_count..hash_count + 1) == Some(" "))
        {
            Some(hash_count as u8)
        } else {
            None
        }
    }

    /// Parse inline markdown content
    fn parse_inline_content(&self, text: &str) -> Vec<Inline> {
        vec![Inline::Text {
            text: text.to_string(),
        }]
    }

    /// Render inline content back to markdown
    fn render_inline_content(&self, content: &[Inline]) -> String {
        content
            .iter()
            .filter_map(|inline| match inline {
                Inline::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_markdown_heading() {
        let parser = MarkdownParser;
        let (block, metadata) = parser.parse("# Hello World", 0).unwrap();

        assert_eq!(metadata.heading_level, Some(1));
        assert!(matches!(
            block,
            Block::Leaf {
                leaf: crate::models::LeafBlock::Heading { .. }
            }
        ));
    }

    #[test]
    fn parse_markdown_paragraph() {
        let parser = MarkdownParser;
        let (block, _) = parser.parse("This is a paragraph", 0).unwrap();

        assert!(matches!(
            block,
            Block::Leaf {
                leaf: crate::models::LeafBlock::Paragraph { .. }
            }
        ));
    }

    #[test]
    fn parse_horizontal_rule() {
        let parser = MarkdownParser;
        let (block, _) = parser.parse("---", 0).unwrap();

        assert!(block.is_horizontal_rule());
    }
}
