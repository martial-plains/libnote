use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ListItem {
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading(u8, Vec<Inline>),
    Quote(Vec<Block>),
    List {
        ordered: bool,
        items: Vec<Block>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Image {
        alt_text: Option<String>,
        src: String,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    MathBlock(String),
    Attachment(Attachment),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        text: Vec<Inline>,
        target: String,
    },
    Image {
        alt_text: Option<String>,
        src: String,
    },
    Code(String),
    Math(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentType {
    Image,
    Audio,
    Video,
    Document,
    Other(String),
}

impl fmt::Display for AttachmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mime = match self {
            AttachmentType::Image => "image/png",
            AttachmentType::Audio => "audio/mpeg",
            AttachmentType::Video => "video/mp4",
            AttachmentType::Document => "application/pdf",
            AttachmentType::Other(mime) => mime.as_str(),
        };
        write!(f, "{}", mime)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub kind: AttachmentType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkTarget {
    Note(String),
    Attachment(String),
}
