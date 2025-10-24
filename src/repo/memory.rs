use core::cell::RefCell;
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    formats::NoteFormat,
    models::Note,
    repo::{NotesRepository, RepoResult},
};

/// In-memory repository for notes
pub struct MemoryNotesRepository {
    notes: RefCell<BTreeMap<String, Note>>,
    format: Arc<dyn NoteFormat>,
}

impl MemoryNotesRepository {
    pub fn new(format: Arc<dyn NoteFormat>) -> Self {
        Self {
            notes: RefCell::new(BTreeMap::new()),
            format,
        }
    }

    /// Parse raw input (Markdown, MsgPack, etc.) and insert as a note
    pub fn insert_raw(&self, raw_data: &[u8], id_hint: Option<&str>) -> RepoResult<String> {
        let note = self.format.deserialize(raw_data, id_hint);
        let id = note.id.clone();
        self.notes.borrow_mut().insert(id.clone(), note);
        Ok(id)
    }

    /// Extract attachments from a stored note
    pub fn get_attachments(&self, note_id: &str) -> RepoResult<Vec<crate::models::Attachment>> {
        if let Some(note) = self.get_note(note_id)? {
            Ok(crate::formats::markdown::extract_attachments(&note.blocks))
        } else {
            Ok(Vec::new())
        }
    }
}

impl NotesRepository for MemoryNotesRepository {
    fn list_notes(&self) -> RepoResult<Vec<Note>> {
        Ok(self.notes.borrow().values().cloned().collect())
    }

    fn get_note(&self, id: &str) -> RepoResult<Option<Note>> {
        Ok(self.notes.borrow().get(id).cloned())
    }

    fn save_note(&mut self, note: &Note) -> RepoResult<()> {
        self.notes
            .borrow_mut()
            .insert(note.id.clone(), note.clone());
        Ok(())
    }

    fn delete_note(&mut self, id: &str) -> RepoResult<()> {
        self.notes.borrow_mut().remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        formats::markdown::MarkdownFormat,
        models::{Attachment, Block, Inline, Note},
    };

    #[test]
    fn memory_repo_insert_raw_and_basic_ops() {
        let format = Arc::new(MarkdownFormat);
        let mut repo = MemoryNotesRepository::new(format);

        let md_data = b"# My Title\nThis is a paragraph.\n![[image.png]]";
        let note_id = repo.insert_raw(md_data, None).unwrap();
        let note = repo.get_note(&note_id).unwrap().unwrap();
        assert_eq!(note.title, "My Title");

        assert!(matches!(note.blocks[0], Block::Paragraph(_)));

        let note2 = Note {
            id: "manual".to_string(),
            title: "Manual Note".to_string(),
            blocks: vec![Block::Paragraph(vec![Inline::Text("Hello".to_string())])],
        };
        repo.save_note(&note2).unwrap();

        let notes = repo.list_notes().unwrap();
        assert_eq!(notes.len(), 2);

        repo.delete_note(&note_id).unwrap();
        assert!(repo.get_note(&note_id).unwrap().is_none());
    }

    #[test]
    fn memory_repo_extract_attachments() {
        let format = Arc::new(MarkdownFormat);
        let repo = MemoryNotesRepository::new(format);

        let md_data = b"# Title\nParagraph with ![](file1.png) and ![](file2.jpg)";

        let note_id = repo.insert_raw(md_data, None).unwrap();

        let attachments: Vec<Attachment> = repo.get_attachments(&note_id).unwrap();
        let names: Vec<&str> = attachments.iter().map(|a| a.name.as_str()).collect();

        assert!(names.contains(&"file1.png"));
        assert!(names.contains(&"file2.jpg"));
        assert_eq!(attachments.len(), 2);
    }
}
