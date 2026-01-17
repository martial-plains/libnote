#![allow(clippy::match_wildcard_for_single_variants)]

use core::fmt;

use serde::{Deserialize, Serialize};

pub type Blocks = Vec<Block>;
pub type Inlines = Vec<Inline>;

/// A key-value attribute pair for `UniFFI` compatibility
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Record)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

/// A definition list item containing term and definition
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Record)]
pub struct DefinitionItem {
    pub term: Inlines,
    pub definition: Blocks,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub blocks: Blocks,
}

/// A note that preserves syntax type information and supports hybrid markup
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HybridNote {
    pub id: String,
    pub title: String,
    /// Blocks with syntax type and round-trip preservation
    pub hybrid_blocks: Vec<crate::parser::HybridBlock>,
}

impl HybridNote {
    /// Create a new hybrid note
    #[must_use]
    pub const fn new(id: String, title: String) -> Self {
        Self {
            id,
            title,
            hybrid_blocks: Vec::new(),
        }
    }

    /// Add a block to the note
    pub fn add_block(&mut self, block: crate::parser::HybridBlock) {
        self.hybrid_blocks.push(block);
    }

    /// Get total number of blocks
    #[must_use]
    pub const fn block_count(&self) -> usize {
        self.hybrid_blocks.len()
    }

    /// Find all headings in the document
    #[must_use]
    pub fn find_headings(&self) -> Vec<(usize, &crate::parser::HybridBlock)> {
        self.hybrid_blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.is_heading())
            .collect()
    }

    /// Find headings at a specific level
    #[must_use]
    pub fn find_headings_at_level(&self, level: u8) -> Vec<(usize, &crate::parser::HybridBlock)> {
        self.hybrid_blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.metadata.heading_level == Some(level))
            .collect()
    }

    /// Find all TODO/DONE items
    #[must_use]
    pub fn find_todos(&self) -> Vec<(usize, &crate::parser::HybridBlock)> {
        self.hybrid_blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.metadata.todo_state.is_some())
            .collect()
    }

    /// Get content at a specific index
    #[must_use]
    pub fn block_at(&self, index: usize) -> Option<&crate::parser::HybridBlock> {
        self.hybrid_blocks.get(index)
    }

    /// Get mutable reference to block at index
    pub fn block_at_mut(&mut self, index: usize) -> Option<&mut crate::parser::HybridBlock> {
        self.hybrid_blocks.get_mut(index)
    }

    /// Remove a block at the given index
    pub fn remove_block(&mut self, index: usize) -> Option<crate::parser::HybridBlock> {
        if index < self.hybrid_blocks.len() {
            Some(self.hybrid_blocks.remove(index))
        } else {
            None
        }
    }

    /// Insert a block at the given index
    pub fn insert_block(&mut self, index: usize, block: crate::parser::HybridBlock) {
        self.hybrid_blocks.insert(index, block);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum ContainerBlock {
    Quote {
        blocks: Blocks,
    },
    List {
        style: ListStyle,
        items: Vec<Blocks>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        alignments: Option<Vec<Alignment>>,
        caption: Option<Vec<Inline>>,
    },
    Div {
        classes: Vec<String>,
        attributes: Vec<Attribute>,
        children: Vec<Block>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum LeafBlock {
    Paragraph {
        content: Inlines,
    },
    Heading {
        level: u8,
        content: Inlines,
    },
    Image {
        alt_text: Option<String>,
        src: String,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    MathBlock {
        content: String,
    },
    HorizontalRule,
    Attachment {
        attachment: Attachment,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum Block {
    Container { container: ContainerBlock },
    Leaf { leaf: LeafBlock },
    DefinitionList { items: Vec<DefinitionItem> },
    FootnoteDefinition { label: String, content: Blocks },
}

impl Block {
    #[must_use]
    pub const fn paragraph(content: Inlines) -> Self {
        Self::Leaf {
            leaf: LeafBlock::Paragraph { content },
        }
    }

    #[must_use]
    pub const fn heading(level: u8, content: Inlines) -> Self {
        Self::Leaf {
            leaf: LeafBlock::Heading { level, content },
        }
    }

    #[must_use]
    pub const fn image(alt_text: Option<String>, src: String) -> Self {
        Self::Leaf {
            leaf: LeafBlock::Image { alt_text, src },
        }
    }

    #[must_use]
    pub const fn code_block(language: Option<String>, content: String) -> Self {
        Self::Leaf {
            leaf: LeafBlock::CodeBlock { language, content },
        }
    }

    #[must_use]
    pub const fn math_block(content: String) -> Self {
        Self::Leaf {
            leaf: LeafBlock::MathBlock { content },
        }
    }

    #[must_use]
    pub const fn horizontal_rule() -> Self {
        Self::Leaf {
            leaf: LeafBlock::HorizontalRule,
        }
    }

    #[must_use]
    pub const fn attachment(attachment: Attachment) -> Self {
        Self::Leaf {
            leaf: LeafBlock::Attachment { attachment },
        }
    }

    #[must_use]
    pub const fn quote(blocks: Blocks) -> Self {
        Self::Container {
            container: ContainerBlock::Quote { blocks },
        }
    }

    #[must_use]
    pub const fn list(style: ListStyle, items: Vec<Blocks>) -> Self {
        Self::Container {
            container: ContainerBlock::List { style, items },
        }
    }

    #[must_use]
    pub fn table(
        headers: Vec<Inlines>,
        rows: Vec<Vec<Inlines>>,
        alignments: Option<Vec<Alignment>>,
        caption: Option<Inlines>,
    ) -> Self {
        let headers_vec = headers.into_iter().collect();
        let rows_vec = rows.into_iter().map(|r| r.into_iter().collect()).collect();

        Self::Container {
            container: ContainerBlock::Table {
                headers: headers_vec,
                rows: rows_vec,
                alignments,
                caption,
            },
        }
    }

    #[must_use]
    pub const fn div(
        classes: Vec<String>,
        attributes: Vec<Attribute>,
        children: Vec<Self>,
    ) -> Self {
        Self::Container {
            container: ContainerBlock::Div {
                classes,
                attributes,
                children,
            },
        }
    }

    #[must_use]
    pub const fn definition_list(items: Vec<DefinitionItem>) -> Self {
        Self::DefinitionList { items }
    }

    #[must_use]
    pub const fn footnote_definition(label: String, content: Blocks) -> Self {
        Self::FootnoteDefinition { label, content }
    }
}

impl Block {
    #[must_use]
    pub const fn as_horizontal_rule(&self) -> Option<LeafBlock> {
        if matches!(
            self,
            Self::Leaf {
                leaf: LeafBlock::HorizontalRule,
            }
        ) {
            Some(LeafBlock::HorizontalRule)
        } else {
            None
        }
    }

    #[must_use]
    pub fn is_horizontal_rule(&self) -> bool {
        self.as_horizontal_rule().is_some()
    }
}

macro_rules! impl_container_helpers {
    ($($variant:ident $( { $($field:ident),* } )?),*) => {
        $(
            impl Block {
                paste::paste! {
                    #[must_use]
                    pub fn [<as_ $variant:snake>](&self) -> Option<ContainerBlock> {
                        if let Block::Container { container: ContainerBlock::$variant $( { $($field),* } )? } = self {
                            Some(ContainerBlock::$variant {
                                $(
                                    $(
                                        $field: $field.clone(),
                                    )*
                                )?
                            })
                        } else {
                            None
                        }
                    }

                    #[must_use]
                    pub fn [<is_ $variant:snake>](&self) -> bool {
                        self.[<as_ $variant:snake>]().is_some()
                    }
                }
            }
        )*
    };
}

macro_rules! impl_leaf_helpers {
    ($($variant:ident $( { $($field:ident),* } )?),*) => {
        $(
            impl Block {
                paste::paste! {
                    pub fn [<as_ $variant:snake>](&self) -> Option<LeafBlock> {
                        if let Block::Leaf { leaf: LeafBlock::$variant $( { $($field),* } )? } = self {
                            Some(LeafBlock::$variant {
                                $(
                                    $(
                                        $field: $field.clone(),
                                    )*
                                )?
                            })
                        } else {
                            None
                        }
                    }

                    pub fn [<is_ $variant:snake>](&self) -> bool {
                        self.[<as_ $variant:snake>]().is_some()
                    }
                }
            }
        )*
    };
}

impl_leaf_helpers!(
    Paragraph { content },
    Heading { level, content },
    Image { alt_text, src },
    CodeBlock { language, content },
    MathBlock { content },
    Attachment { attachment }
);

impl_container_helpers!(
    Quote { blocks },
    List { style, items },
    Table {
        headers,
        rows,
        alignments,
        caption
    },
    Div {
        classes,
        attributes,
        children
    }
);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum ListStyle {
    Unordered { bullet: u8 },
    Ordered { numbering: Numbering },
}

impl ListStyle {
    #[must_use]
    pub const fn is_ordered(&self) -> bool {
        match self {
            Self::Ordered { .. } => true,
            Self::Unordered { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum NumberingType {
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum NumberingStyle {
    Dot,
    Paren,
    ZeroPadded,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Record)]
pub struct Numbering {
    pub kind: NumberingType,
    pub style: NumberingStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, uniffi::Enum)]
pub enum Inline {
    Text {
        text: String,
    },
    Bold {
        content: Vec<Inline>,
    },
    Italic {
        content: Vec<Inline>,
    },
    Strikethrough {
        content: Vec<Inline>,
    },

    Link {
        text: Vec<Inline>,
        target: String,
    },

    Image {
        alt_text: Option<String>,
        src: String,
    },

    Code {
        code: String,
    },
    Math {
        content: String,
    },

    LineBreak,

    Superscript {
        content: Vec<Inline>,
    },
    Subscript {
        content: Vec<Inline>,
    },

    FootnoteReference {
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
pub enum AttachmentType {
    Image,
    Audio,
    Video,
    Document,
    Other { mime: String },
}

impl fmt::Display for AttachmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mime = match self {
            Self::Image => "image/png",
            Self::Audio => "audio/mpeg",
            Self::Video => "video/mp4",
            Self::Document => "application/pdf",
            Self::Other { mime } => mime.as_str(),
        };
        write!(f, "{mime}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Record)]
pub struct Attachment {
    pub name: String,
    pub src: String,
    pub kind: AttachmentType,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, uniffi::Enum)]
pub enum DecimalStyle {
    Dot,
    Paren,
    ZeroPadded,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum LinkTarget {
    Note(String),
    Attachment(String),
}

/// Document format preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum DocumentFormat {
    /// Pure AST representation (standard Note)
    Abstract,
    /// Hybrid representation preserving multiple markup syntaxes
    Hybrid,
}

/// A document that can be either abstract or hybrid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Document {
    /// Standard AST-based note
    Standard(Note),
    /// Hybrid note with mixed markup syntaxes
    Hybrid(HybridNote),
}

impl Document {
    /// Create a new standard document
    #[must_use]
    pub const fn standard(id: String, title: String) -> Self {
        Self::Standard(Note {
            id,
            title,
            blocks: Vec::new(),
        })
    }

    /// Create a new hybrid document
    #[must_use]
    pub const fn hybrid(id: String, title: String) -> Self {
        Self::Hybrid(HybridNote {
            id,
            title,
            hybrid_blocks: Vec::new(),
        })
    }

    /// Get document ID
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Standard(note) => &note.id,
            Self::Hybrid(note) => &note.id,
        }
    }

    /// Get document title
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Standard(note) => &note.title,
            Self::Hybrid(note) => &note.title,
        }
    }

    /// Get document format
    #[must_use]
    pub const fn format(&self) -> DocumentFormat {
        match self {
            Self::Standard(_) => DocumentFormat::Abstract,
            Self::Hybrid(_) => DocumentFormat::Hybrid,
        }
    }

    /// Convert to hybrid format (if not already)
    #[must_use]
    pub const fn as_hybrid(&self) -> Option<&HybridNote> {
        match self {
            Self::Hybrid(note) => Some(note),
            _ => None,
        }
    }

    /// Get mutable reference to hybrid format (if applicable)
    pub const fn as_hybrid_mut(&mut self) -> Option<&mut HybridNote> {
        match self {
            Self::Hybrid(note) => Some(note),
            _ => None,
        }
    }

    /// Convert to standard format (if not already)
    #[must_use]
    pub const fn as_standard(&self) -> Option<&Note> {
        match self {
            Self::Standard(note) => Some(note),
            _ => None,
        }
    }

    /// Get mutable reference to standard format (if applicable)
    pub const fn as_standard_mut(&mut self) -> Option<&mut Note> {
        match self {
            Self::Standard(note) => Some(note),
            _ => None,
        }
    }
}
