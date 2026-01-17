#![allow(clippy::missing_panics_doc)]

use super::span::TextSpan;
use crate::models::{Block, Inlines, LeafBlock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub id: NodeId,
    pub block: Block,
    pub span: TextSpan,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub id: NodeId,
    pub level: u8, // 0 = root
    pub title: Option<Inlines>,
    pub blocks: Vec<BlockNode>,
    pub children: Vec<Self>,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub root: Section,
}

#[must_use] 
pub fn build_section_tree(blocks: Vec<BlockNode>) -> Section {
    let root = Section {
        id: NodeId(0),
        level: 0,
        title: None,
        blocks: Vec::new(),
        children: Vec::new(),
    };

    let mut stack = vec![root];

    for node in blocks {
        if let Block::Leaf {
            leaf: LeafBlock::Heading { level, content },
        } = &node.block
        {
            let level = *level;

            while stack.len() > 1 && stack.last().unwrap().level >= level {
                let finished_section = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(finished_section);
            }

            stack.push(Section {
                id: node.id,
                level,
                title: Some(content.clone()),
                blocks: Vec::new(),
                children: Vec::new(),
            });
        } else {
            stack.last_mut().unwrap().blocks.push(node);
        }
    }

    while stack.len() > 1 {
        let finished_section = stack.pop().unwrap();
        stack.last_mut().unwrap().children.push(finished_section);
    }

    stack.pop().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, Inline, LeafBlock};

    fn node(id: u64, block: Block) -> BlockNode {
        BlockNode {
            id: NodeId(id),
            block,
            span: TextSpan { start: 0, end: 0 },
        }
    }

    fn heading(id: u64, level: u8, text: &str) -> BlockNode {
        node(
            id,
            Block::Leaf {
                leaf: LeafBlock::Heading {
                    level,
                    content: vec![Inline::Text {
                        text: text.to_string(),
                    }],
                },
            },
        )
    }

    fn para(id: u64, text: &str) -> BlockNode {
        node(
            id,
            Block::Leaf {
                leaf: LeafBlock::Paragraph {
                    content: vec![Inline::Text {
                        text: text.to_string(),
                    }],
                },
            },
        )
    }

    #[test]
    fn single_heading_with_paragraph() {
        let blocks = vec![heading(1, 1, "Title"), para(2, "Hello world")];

        let doc = build_section_tree(blocks);

        assert_eq!(doc.children.len(), 1);

        let section = &doc.children[0];
        assert_eq!(section.level, 1);
        assert_eq!(section.blocks.len(), 1);

        match &section.blocks[0].block {
            Block::Leaf {
                leaf: LeafBlock::Paragraph { .. },
            } => {}
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn nested_headings() {
        let blocks = vec![
            heading(1, 1, "H1"),
            para(2, "A"),
            heading(3, 2, "H2"),
            para(4, "B"),
            heading(5, 1, "H1 again"),
            para(6, "C"),
        ];

        let doc = build_section_tree(blocks);

        assert_eq!(doc.children.len(), 2);

        let first = &doc.children[0];
        assert_eq!(first.children.len(), 1);
        assert_eq!(first.blocks.len(), 1);

        let nested = &first.children[0];
        assert_eq!(nested.level, 2);
        assert_eq!(nested.blocks.len(), 1);

        let second = &doc.children[1];
        assert_eq!(second.blocks.len(), 1);
    }

    #[test]
    fn heading_level_skip_is_allowed() {
        let blocks = vec![
            heading(1, 1, "H1"),
            heading(2, 3, "H3"),
            para(3, "Deep content"),
        ];

        let doc = build_section_tree(blocks);

        let h1 = &doc.children[0];
        assert_eq!(h1.children.len(), 1);

        let h3 = &h1.children[0];
        assert_eq!(h3.level, 3);
        assert_eq!(h3.blocks.len(), 1);
    }

    #[test]
    fn content_before_heading_goes_in_root() {
        let blocks = vec![para(1, "Intro"), heading(2, 1, "Title"), para(3, "Body")];

        let doc = build_section_tree(blocks);

        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.children.len(), 1);

        let root_para = &doc.blocks[0];
        match root_para.block {
            Block::Leaf {
                leaf: LeafBlock::Paragraph { .. },
            } => {}
            _ => panic!("expected root paragraph"),
        }
    }

    #[test]
    fn node_ids_are_preserved() {
        let blocks = vec![heading(10, 1, "Title"), para(11, "Text")];

        let doc = build_section_tree(blocks);

        let section = &doc.children[0];
        assert_eq!(section.id, NodeId(10));

        let block = &section.blocks[0];
        assert_eq!(block.id, NodeId(11));
    }

    #[test]
    fn many_headings_no_panic() {
        let mut blocks = Vec::new();

        for i in 0..100 {
            blocks.push(heading(i * 2, 1, "H"));
            blocks.push(para(i * 2 + 1, "P"));
        }

        let doc = build_section_tree(blocks);
        assert_eq!(doc.children.len(), 100);
    }
}
