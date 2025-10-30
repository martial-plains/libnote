use core::fmt;

use serde::{Deserialize, Serialize};

pub type Blocks = Vec<Block>;
pub type Inlines = Vec<Inline>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub blocks: Blocks,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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
        attributes: Vec<(String, String)>,
        children: Vec<Block>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Block {
    Container { container: ContainerBlock },
    Leaf { leaf: LeafBlock },
    DefinitionList { items: Vec<(Inlines, Blocks)> },
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
        attributes: Vec<(String, String)>,
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
    pub const fn definition_list(items: Vec<(Inlines, Blocks)>) -> Self {
        Self::DefinitionList { items }
    }

    #[must_use]
    pub const fn footnote_definition(label: String, content: Blocks) -> Self {
        Self::FootnoteDefinition { label, content }
    }
}

macro_rules! impl_container_helpers {
    ($($variant:ident $( { $($field:ident),* } )?),*) => {
        $(
            impl Block {
                paste::paste! {
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
    HorizontalRule,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ListStyle {
    Unordered { bullet: char },
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum NumberingType {
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum NumberingStyle {
    Dot,
    Paren,
    ZeroPadded,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Numbering {
    pub kind: NumberingType,
    pub style: NumberingStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub src: String,
    pub kind: AttachmentType,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum DecimalStyle {
    Dot,
    Paren,
    ZeroPadded,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkTarget {
    Note(String),
    Attachment(String),
}
