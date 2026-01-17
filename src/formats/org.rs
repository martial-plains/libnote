#![allow(
    clippy::missing_panics_doc,
    clippy::type_complexity,
    clippy::cast_possible_truncation,
    clippy::match_wildcard_for_single_variants
)]

// formats/org/mod.rs

use crate::formats::{NoteMetadata, NoteSerialization};
use crate::models::{Block, ContainerBlock, Inline, LeafBlock, LinkTarget, Note};

/// Entry point for Org serialization/deserialization
#[derive(Debug, uniffi::Object)]
pub struct OrgFormat;

impl NoteSerialization for OrgFormat {
    fn deserialize(&self, data: &[u8], id_hint: Option<&str>) -> Note {
        let text = std::str::from_utf8(data).unwrap_or("");
        let doc = parser::parse_org(text);
        lower::org_to_note(doc, id_hint)
    }

    fn serialize(&self, note: &Note) -> Vec<u8> {
        serializer::note_to_org(note).into_bytes()
    }
}

impl NoteMetadata for OrgFormat {
    fn extract_tags(&self, content: &str) -> Vec<String> {
        let doc = parser::parse_org(content);
        metadata::extract_tags(&doc)
    }

    fn extract_links(
        &self,
        note: &Note,
        _attachments: &[crate::models::Attachment],
    ) -> Vec<crate::models::LinkTarget> {
        fn walk_blocks(blocks: &[crate::models::Block], out: &mut Vec<crate::models::LinkTarget>) {
            for block in blocks {
                match block {
                    Block::Leaf {
                        leaf: LeafBlock::Paragraph { content } | LeafBlock::Heading { content, .. },
                    } => {
                        for inline in content {
                            if let Inline::Link { target, .. } = inline {
                                out.push(LinkTarget::Note(target.clone()));
                            }
                        }
                    }
                    Block::Container { container } => match container {
                        ContainerBlock::Quote { blocks } => {
                            walk_blocks(blocks, out);
                        }
                        ContainerBlock::List { items, .. } => {
                            for item in items {
                                walk_blocks(item, out);
                            }
                        }
                        ContainerBlock::Table { rows, .. } => {
                            for row in rows {
                                for cell in row {
                                    for inline in cell {
                                        if let Inline::Link { target, .. } = inline {
                                            out.push(LinkTarget::Note(target.clone()));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        let mut links = Vec::new();
        walk_blocks(&note.blocks, &mut links);
        links
    }
}

pub mod model {
    use crate::models::Inline;
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OrgDocument {
        pub nodes: Vec<OrgNode>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OrgNode {
        pub id: Option<String>,
        pub level: u8,
        pub title: Vec<Inline>,
        pub todo: Option<String>,
        pub tags: Vec<String>,
        pub properties: HashMap<String, String>,
        pub body: Vec<crate::models::Block>,
        pub children: Vec<Self>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OrgTable {
        pub headers: Vec<Vec<Inline>>,
        pub rows: Vec<Vec<Vec<Inline>>>,
        pub formulas: Vec<TableFormula>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TableFormula {
        pub formula: String,
        pub target_cells: Vec<(usize, usize)>,
    }
}

/// --- parser.rs ---
pub mod parser {
    use super::model::{OrgDocument, OrgNode};
    use crate::{
        formats::org::model::TableFormula,
        models::{Block, Inline},
    };
    use std::collections::HashMap;

    #[must_use]
    pub fn parse_org(input: &str) -> OrgDocument {
        let mut root: Vec<OrgNode> = Vec::new();
        let mut stack: Vec<OrgNode> = Vec::new();
        let mut lines = input.lines().peekable();

        while let Some(line) = lines.next() {
            if let Some((level, todo, title, tags)) = parse_heading(line) {
                let node = OrgNode {
                    id: None,
                    level,
                    title,
                    todo,
                    tags,
                    properties: HashMap::new(),
                    body: Vec::new(),
                    children: Vec::new(),
                };

                while let Some(top) = stack.last() {
                    if top.level < level {
                        break;
                    }
                    let finished = stack.pop().unwrap();
                    attach_node(&mut root, &mut stack, finished);
                }

                stack.push(node);
            } else if let Some((k, v)) = parse_property(line) {
                if let Some(last) = stack.last_mut() {
                    last.properties.insert(k, v);
                }
            } else if let Some(node) = stack.last_mut() {
                node.body.push(parse_block(line, &mut lines));
            }
        }

        while let Some(node) = stack.pop() {
            attach_node(&mut root, &mut stack, node);
        }

        OrgDocument { nodes: root }
    }

    fn parse_heading(line: &str) -> Option<(u8, Option<String>, Vec<Inline>, Vec<String>)> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('*') {
            return None;
        }

        let level = trimmed.chars().take_while(|c| *c == '*').count();
        let rest = trimmed[level..].trim();

        let mut todo = None;
        let mut title = Vec::new();
        let mut tags = Vec::new();

        let parts: Vec<&str> = rest.split_whitespace().collect();
        let mut i = 0;

        if parts
            .first()
            .is_some_and(|s| s.chars().all(char::is_uppercase))
        {
            todo = Some(parts[0].to_string());
            i += 1;
        }

        while i < parts.len() {
            let part = parts[i];
            if part.starts_with(':') && part.ends_with(':') {
                tags = part
                    .trim_matches(':')
                    .split(':')
                    .map(ToString::to_string)
                    .collect();
                break;
            }
            title.push(Inline::Text {
                text: part.to_string(),
            });
            i += 1;
        }

        Some((level as u8, todo, title, tags))
    }

    fn parse_property(line: &str) -> Option<(String, String)> {
        if !line.starts_with(':') {
            return None;
        }
        let parts: Vec<&str> = line.trim_matches(':').splitn(2, ':').collect();
        if parts.len() == 2 {
            Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
        } else {
            None
        }
    }

    fn parse_block(line: &str, lines_iter: &mut std::iter::Peekable<std::str::Lines>) -> Block {
        if line.trim().starts_with('|') {
            let mut rows = Vec::new();
            while let Some(&next_line) = lines_iter.peek() {
                if next_line.trim().starts_with('|') {
                    rows.push(parse_table_row(next_line));
                    lines_iter.next();
                } else {
                    break;
                }
            }

            return Block::Container {
                container: crate::models::ContainerBlock::Table {
                    headers: rows.first().cloned().unwrap_or_default(),
                    rows,
                    alignments: None,
                    caption: None,
                },
            };
        }

        Block::Leaf {
            leaf: crate::models::LeafBlock::Paragraph {
                content: vec![crate::models::Inline::Text {
                    text: line.to_string(),
                }],
            },
        }
    }

    fn parse_table_row(line: &str) -> Vec<Vec<Inline>> {
        line.trim()
            .trim_matches('|')
            .split('|')
            .map(|cell| {
                vec![Inline::Text {
                    text: cell.trim().to_string(),
                }]
            })
            .collect()
    }

    #[allow(dead_code)]
    fn parse_table_formulas(line: &str) -> Vec<TableFormula> {
        let line = line.trim().trim_start_matches("#+TBLFM:").trim();
        line.split(';')
            .map(|f| TableFormula {
                formula: f.to_string(),
                target_cells: Vec::new(),
            })
            .collect()
    }

    fn attach_node(root: &mut Vec<OrgNode>, stack: &mut [OrgNode], node: OrgNode) {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            root.push(node);
        }
    }
}

/// --- lower.rs ---
pub mod lower {
    use super::model::OrgDocument;
    use crate::models::{Block, Note};

    #[must_use]
    pub fn org_to_note(doc: OrgDocument, id_hint: Option<&str>) -> Note {
        let mut blocks = Vec::new();

        for node in doc.nodes {
            lower_node(&node, &mut blocks);
        }

        Note {
            id: id_hint.unwrap_or("org").to_string(),
            title: extract_title(&blocks),
            blocks,
        }
    }

    fn lower_node(node: &super::model::OrgNode, out: &mut Vec<Block>) {
        out.push(Block::heading(node.level, node.title.clone()));
        out.extend(node.body.clone());
        for child in &node.children {
            lower_node(child, out);
        }
    }

    fn extract_title(blocks: &[Block]) -> String {
        for block in blocks {
            if let Block::Leaf {
                leaf: crate::models::LeafBlock::Heading { content, .. },
            } = block
            {
                return content
                    .iter()
                    .filter_map(|i| {
                        if let crate::models::Inline::Text { text } = i {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
            }
        }
        "Untitled".to_string()
    }
}

pub mod serializer {
    use crate::models::{Block, ContainerBlock, Inline, LeafBlock, Note};

    #[must_use]
    pub fn note_to_org(note: &Note) -> String {
        let mut out = String::new();
        for block in &note.blocks {
            write_block(block, &mut out);
        }
        out
    }

    fn write_block(block: &Block, out: &mut String) {
        match block {
            Block::Leaf { leaf } => match leaf {
                LeafBlock::Paragraph { content } => {
                    for inline in content {
                        write_inline(inline, out);
                    }
                    out.push('\n');
                }
                LeafBlock::Heading { level, content } => {
                    out.push_str(&"*".repeat(*level as usize));
                    out.push(' ');
                    for inline in content {
                        write_inline(inline, out);
                    }
                    out.push('\n');
                }
                _ => {}
            },
            Block::Container {
                container: ContainerBlock::Quote { blocks },
            } => {
                out.push_str("#+begin_quote\n");
                for b in blocks {
                    write_block(b, out);
                }
                out.push_str("#+end_quote\n");
            }
            _ => {}
        }
    }

    fn write_inline(inline: &Inline, out: &mut String) {
        match inline {
            Inline::Text { text } => out.push_str(text),
            Inline::Bold { content } => {
                out.push('*');
                for c in content {
                    write_inline(c, out);
                }
                out.push('*');
            }
            Inline::Italic { content } => {
                out.push('/');
                for c in content {
                    write_inline(c, out);
                }
                out.push('/');
            }
            Inline::Strikethrough { content } => {
                out.push('+');
                for c in content {
                    write_inline(c, out);
                }
                out.push('+');
            }
            Inline::Code { code } => {
                out.push('~');
                out.push_str(code);
                out.push('~');
            }
            Inline::Math { content } => {
                out.push_str("\\(");
                out.push_str(content);
                out.push_str("\\)");
            }
            _ => {}
        }
    }
}

pub mod metadata {
    use super::model::OrgDocument;

    #[must_use]
    pub fn extract_tags(doc: &OrgDocument) -> Vec<String> {
        fn visit_nodes(nodes: &[super::model::OrgNode], out: &mut Vec<String>) {
            for node in nodes {
                out.extend(node.tags.clone());
                visit_nodes(&node.children, out);
            }
        }

        let mut tags = Vec::new();
        visit_nodes(&doc.nodes, &mut tags);
        tags
    }
}
