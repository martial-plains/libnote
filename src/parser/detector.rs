//! Block detection module for identifying block boundaries by syntax markers
#![allow(clippy::unused_self)]

use super::SyntaxKind;

/// A raw block detected by line scanning
#[derive(Debug, Clone)]
pub struct SyntaxBlock {
    /// The syntax kind of this block
    pub syntax: SyntaxKind,
    /// Language hint for code blocks
    pub language: Option<String>,
    /// Raw text content
    pub content: String,
    /// Starting line number (0-based)
    pub start_line: usize,
    /// Ending line number (0-based, inclusive)
    pub end_line: usize,
}

impl SyntaxBlock {
    /// Get number of lines in this block
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.end_line - self.start_line + 1
    }
}

/// Configuration for block detection
#[derive(Debug, Clone, uniffi::Record)]
pub struct DetectionConfig {
    /// Markers for Markdown fenced code blocks
    pub markdown_code_fence: String,
    /// Markers for Org-mode blocks
    pub org_block_markers: Vec<String>,
    /// Markers for LaTeX environments
    pub latex_markers: Vec<String>,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            markdown_code_fence: "```".to_string(),
            org_block_markers: vec!["#+BEGIN_".to_string(), "#+END_".to_string()],
            latex_markers: vec!["$$".to_string(), "\\[".to_string(), "\\]".to_string()],
        }
    }
}

/// Block detector that identifies blocks by syntax markers
#[derive(Debug)]
pub struct BlockDetector {
    #[allow(dead_code)]
    config: DetectionConfig,
}

impl BlockDetector {
    /// Create a new detector with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: DetectionConfig::default(),
        }
    }

    /// Create a detector with custom configuration
    #[must_use]
    pub const fn with_config(config: DetectionConfig) -> Self {
        Self { config }
    }

    /// Detect blocks in the given document text
    #[must_use]
    pub fn detect(&self, text: &str) -> Vec<SyntaxBlock> {
        let lines: Vec<&str> = text.lines().collect();
        let mut blocks = Vec::new();
        let mut current_pos = 0;

        while current_pos < lines.len() {
            if let Some((block, new_pos)) = self.scan_block(&lines, current_pos) {
                blocks.push(block);
                current_pos = new_pos;
            } else {
                current_pos += 1;
            }
        }

        blocks
    }

    /// Scan for a block starting at the given line
    fn scan_block(&self, lines: &[&str], start: usize) -> Option<(SyntaxBlock, usize)> {
        let line = lines[start];

        if line.trim_start().starts_with("#+BEGIN_") {
            return self.scan_org_block(lines, start);
        }

        if line.trim().starts_with("```") {
            return self.scan_markdown_code_block(lines, start);
        }

        if line.contains("$$") {
            return self.scan_latex_block(lines, start);
        }

        if !line.trim().is_empty() {
            return self.scan_markdown_block(lines, start);
        }

        None
    }

    /// Scan an Org-mode block (#+BEGIN_...#+END_...)
    fn scan_org_block(&self, lines: &[&str], start: usize) -> Option<(SyntaxBlock, usize)> {
        let first_line = lines[start];
        let trimmed = first_line.trim_start();

        if !trimmed.starts_with("#+BEGIN_") {
            return None;
        }

        let after_begin = trimmed.strip_prefix("#+BEGIN_")?;
        let block_type = after_begin.split_whitespace().next()?.to_uppercase();

        let end_marker = format!("#+END_{block_type}");

        for (i, line) in lines[start + 1..].iter().enumerate() {
            if line.trim_start().starts_with(&end_marker) {
                let content = lines[start..=start + i + 1].join("\n");
                return Some((
                    SyntaxBlock {
                        syntax: SyntaxKind::Org,
                        language: None,
                        content,
                        start_line: start,
                        end_line: start + i + 1,
                    },
                    start + i + 2,
                ));
            }
        }

        None
    }

    /// Scan a Markdown code block (```language ... ```)
    fn scan_markdown_code_block(
        &self,
        lines: &[&str],
        start: usize,
    ) -> Option<(SyntaxBlock, usize)> {
        let first_line = lines[start].trim();
        let language = first_line
            .strip_prefix("```")
            .and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    trimmed.split_whitespace().next()
                }
            })
            .map(std::string::ToString::to_string);

        for (i, line) in lines[start + 1..].iter().enumerate() {
            if line.trim().starts_with("```") {
                let content = lines[start..=start + i + 1].join("\n");
                return Some((
                    SyntaxBlock {
                        syntax: SyntaxKind::Code,
                        language,
                        content,
                        start_line: start,
                        end_line: start + i + 1,
                    },
                    start + i + 2,
                ));
            }
        }

        None
    }

    /// Scan a LaTeX block ($$...$$)
    fn scan_latex_block(&self, lines: &[&str], start: usize) -> Option<(SyntaxBlock, usize)> {
        let first_line = lines[start];
        let starts_with_dollars = first_line.contains("$$");

        if !starts_with_dollars {
            return None;
        }

        let line_dollar_count = first_line.matches("$$").count();

        if line_dollar_count >= 2 {
            return Some((
                SyntaxBlock {
                    syntax: SyntaxKind::LaTeX,
                    language: None,
                    content: first_line.to_string(),
                    start_line: start,
                    end_line: start,
                },
                start + 1,
            ));
        }

        for (i, line) in lines[start + 1..].iter().enumerate() {
            if line.contains("$$") {
                let content = lines[start..=start + i + 1].join("\n");
                return Some((
                    SyntaxBlock {
                        syntax: SyntaxKind::LaTeX,
                        language: None,
                        content,
                        start_line: start,
                        end_line: start + i + 1,
                    },
                    start + i + 2,
                ));
            }
        }

        None
    }

    /// Scan a regular Markdown block (until empty line or another block marker)
    fn scan_markdown_block(&self, lines: &[&str], start: usize) -> Option<(SyntaxBlock, usize)> {
        let mut end = start;

        for (i, line) in lines[start..].iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.is_empty() && i > 0 {
                end = start + i - 1;
                break;
            }

            if i > 0
                && (trimmed.starts_with("#+BEGIN_")
                    || trimmed.starts_with("```")
                    || trimmed.contains("$$"))
            {
                end = start + i - 1;
                break;
            }

            end = start + i;
        }

        if end >= start {
            let content = lines[start..=end].join("\n");
            return Some((
                SyntaxBlock {
                    syntax: SyntaxKind::Markdown,
                    language: None,
                    content,
                    start_line: start,
                    end_line: end,
                },
                end + 1,
            ));
        }

        None
    }
}

impl Default for BlockDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_single_markdown_block() {
        let text = "This is a paragraph\nwith multiple lines";
        let detector = BlockDetector::new();
        let blocks = detector.detect(text);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].syntax, SyntaxKind::Markdown);
    }

    #[test]
    fn detect_code_block() {
        let text = "Some text\n\n```rust\nfn main() {}\n```";
        let detector = BlockDetector::new();
        let blocks = detector.detect(text);

        assert!(blocks.len() >= 2);
        assert!(blocks.iter().any(|b| matches!(b.syntax, SyntaxKind::Code)));
    }

    #[test]
    fn detect_org_block() {
        let text = "#+BEGIN_SRC python\nprint('hello')\n#+END_SRC";
        let detector = BlockDetector::new();
        let blocks = detector.detect(text);

        assert!(blocks.iter().any(|b| b.syntax == SyntaxKind::Org));
    }

    #[test]
    fn detect_latex_block() {
        let text = "Some text\n\n$$E = mc^2$$";
        let detector = BlockDetector::new();
        let blocks = detector.detect(text);

        assert!(blocks.iter().any(|b| b.syntax == SyntaxKind::LaTeX));
    }
}
