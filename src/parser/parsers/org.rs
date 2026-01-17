//! Org-mode parser implementation
#![allow(clippy::unused_self)]

use crate::models::{Block, Inline};
use crate::parser::{BlockMetadata, SyntaxKind};

use super::super::interface::{ParseResult, Parser};

/// Parser for Org-mode syntax
pub struct OrgParser;

impl Parser for OrgParser {
    fn syntax_kind(&self) -> SyntaxKind {
        SyntaxKind::Org
    }

    fn parse(&self, raw_text: &str, _line_offset: usize) -> ParseResult<(Block, BlockMetadata)> {
        let mut metadata = BlockMetadata::default();
        let lines: Vec<&str> = raw_text.lines().collect();

        if lines.is_empty() {
            return Ok((Block::paragraph(vec![]), metadata));
        }

        let first_line = lines[0].trim_start();

        if let Some((level, title)) = self.parse_heading_line(first_line) {
            metadata.heading_level = Some(level);

            if title.starts_with("TODO ") {
                metadata.todo_state = Some("TODO".to_string());
            } else if title.starts_with("DONE ") {
                metadata.todo_state = Some("DONE".to_string());
            }

            let content = vec![Inline::Text {
                text: title,
            }];

            return Ok((Block::heading(level, content), metadata));
        }

        if first_line.starts_with("#+BEGIN_SRC")
            && let Some(language) = first_line.strip_prefix("#+BEGIN_SRC").map(str::trim) {
                let code_lines = &lines[1..lines.len().saturating_sub(1)];
                let code_content = code_lines.join("\n");
                return Ok((
                    Block::code_block(
                        if language.is_empty() { None } else { Some(language.to_string()) },
                        code_content,
                    ),
                    metadata,
                ));
            }

        let content = vec![Inline::Text {
            text: raw_text.to_string(),
        }];
        Ok((Block::paragraph(content), metadata))
    }

    fn render(&self, block: &Block, metadata: &BlockMetadata) -> String {
        match block {
            Block::Leaf {
                leaf: crate::models::LeafBlock::Heading { level, content },
            } => {
                let stars = "*".repeat(*level as usize);
                let text = self.render_inline_content(content);
                let todo_prefix = metadata
                    .todo_state
                    .as_ref()
                    .map(|s| format!("{s} "))
                    .unwrap_or_default();
                format!("{stars} {todo_prefix}{text}")
            }
            Block::Leaf {
                leaf: crate::models::LeafBlock::CodeBlock { language, content },
            } => {
                let lang = language.as_deref().unwrap_or("");
                format!("#+BEGIN_SRC {lang}\n{content}\n#+END_SRC")
            }
            Block::Leaf {
                leaf: crate::models::LeafBlock::Paragraph { content },
            } => self.render_inline_content(content),
            _ => String::new(),
        }
    }

    fn can_handle(&self, text: &str) -> bool {
        let trimmed = text.trim_start();
        trimmed.starts_with('*')
            || trimmed.starts_with("#+BEGIN_")
            || trimmed.starts_with("#+TITLE:")
            || trimmed.starts_with("#+AUTHOR:")
    }
}

impl OrgParser {
    /// Parse Org-mode heading syntax (e.g., "* Heading" or "** Subheading")
    #[allow(clippy::cast_possible_truncation)]
    fn parse_heading_line(&self, line: &str) -> Option<(u8, String)> {
        if !line.starts_with('*') {
            return None;
        }

        let star_count = line.chars().take_while(|c| *c == '*').count();
        if star_count == 0 || star_count >= 7 {
            return None;
        }

        let rest = line.get(star_count..).and_then(|s| s.strip_prefix(' '))?;

        Some((star_count as u8, rest.trim().to_string()))
    }

    /// Render inline content
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
    fn parse_org_heading() {
        let parser = OrgParser;
        let (_block, metadata) = parser.parse("* Top Level", 0).unwrap();

        assert_eq!(metadata.heading_level, Some(1));
    }

    #[test]
    fn parse_org_todo_heading() {
        let parser = OrgParser;
        let (_block, metadata) = parser.parse("* TODO Task Title", 0).unwrap();

        assert_eq!(metadata.heading_level, Some(1));
        assert_eq!(metadata.todo_state, Some("TODO".to_string()));
    }

    #[test]
    fn parse_org_code_block() {
        let parser = OrgParser;
        let (block, _) = parser.parse("#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC", 0).unwrap();

        assert!(matches!(
            block,
            Block::Leaf {
                leaf: crate::models::LeafBlock::CodeBlock { .. }
            }
        ));
    }
}
