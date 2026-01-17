use std::fmt::Debug;

use crate::models::{Attachment, LinkTarget, Note};

pub mod markdown;
pub mod org;

#[uniffi::trait_interface]
pub trait NoteSerialization: Send + Sync + Debug {
    /// Deserialize bytes into a Note
    fn deserialize(&self, data: &[u8], id_hint: Option<&str>) -> Note;

    /// Serialize a Note into bytes
    fn serialize(&self, note: &Note) -> Vec<u8>;
}

#[uniffi::trait_interface]
pub trait NoteMetadata: Send + Sync + Debug {
    /// Extract tags from content if supported
    fn extract_tags(&self, _content: &str) -> Vec<String> {
        Vec::new()
    }

    /// Extract links/backlinks from content if supported
    fn extract_links(&self, _note: &Note, _attachments: &[Attachment]) -> Vec<LinkTarget> {
        Vec::new()
    }
}
