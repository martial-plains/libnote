use std::{fmt::Debug, sync::Arc};

use crate::{
    formats::{NoteSerialization, markdown::extract_attachments},
    models::{Attachment, Note},
    repo::{NotesRepository, RepoResult},
};

#[uniffi::trait_interface]
pub trait FileProvider: Send + Sync + Debug {
    /// Read the raw bytes of a file relative to the vault
    fn read(&self, path: &str) -> Option<Vec<u8>>;

    /// Write raw bytes to a file relative to the vault
    fn write(&mut self, path: &str, data: &[u8]) -> bool;

    /// Delete a file by path
    fn delete(&mut self, path: &str) -> bool;

    /// List all note IDs (or file names) in the provider
    fn list(&self) -> Vec<String>;
}

/// File-based repository
#[derive(Debug, uniffi::Object)]
pub struct FileNotesRepository {
    provider: Box<dyn FileProvider>,
    formats: Arc<dyn NoteSerialization>,
}

impl FileNotesRepository {
    #[uniffi::constructor]
    pub fn new(provider: Box<dyn FileProvider>, formats: Arc<dyn NoteSerialization>) -> Self {
        Self { provider, formats }
    }

    /// Extract all attachments from a note using the format's parser.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the note from the repository fails
    pub fn get_attachments(&self, note_id: &str) -> RepoResult<Vec<Attachment>> {
        if let Some(note) = self.get_note(note_id)? {
            Ok(extract_attachments(&note.blocks))
        } else {
            Ok(Vec::new())
        }
    }
}

impl NotesRepository for FileNotesRepository {
    fn list_notes(&self) -> RepoResult<Vec<Note>> {
        let mut notes = Vec::new();
        for id in self.provider.list() {
            if let Some(bytes) = self.provider.read(&id) {
                notes.push(self.formats.deserialize(&bytes, Some(&id)));
            }
        }
        Ok(notes)
    }

    fn get_note(&self, id: &str) -> RepoResult<Option<Note>> {
        self.provider.read(id).map_or_else(
            || Ok(None),
            |bytes| Ok(Some(self.formats.deserialize(&bytes, Some(id)))),
        )
    }

    fn save_note(&mut self, note: &Note) -> RepoResult<()> {
        let data = self.formats.serialize(note);
        if self.provider.write(&note.id, &data) {
            Ok(())
        } else {
            Err(Box::from("Failed to write note"))
        }
    }

    fn delete_note(&mut self, id: &str) -> RepoResult<()> {
        if self.provider.delete(id) {
            Ok(())
        } else {
            Err(Box::from("Failed to delete note"))
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;
    use crate::models::{Block, Inline, Note};

    #[derive(Debug)]
    struct MockProvider {
        files: HashMap<String, Vec<u8>>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }
    }

    impl FileProvider for MockProvider {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.files.get(path).cloned()
        }

        fn write(&mut self, path: &str, data: &[u8]) -> bool {
            self.files.insert(path.to_string(), data.to_vec());
            true
        }

        fn delete(&mut self, path: &str) -> bool {
            self.files.remove(path).is_some()
        }

        fn list(&self) -> Vec<String> {
            self.files.keys().cloned().collect()
        }
    }

    #[derive(Debug)]
    struct MockFormat;

    impl NoteSerialization for MockFormat {
        fn serialize(&self, note: &Note) -> Vec<u8> {
            serde_cbor::to_vec(note).unwrap()
        }

        fn deserialize(&self, data: &[u8], id: Option<&str>) -> Note {
            let mut note: Note = serde_cbor::from_slice(data).unwrap();
            if let Some(id) = id {
                note.id = id.to_string();
            }
            note
        }
    }

    #[test]
    fn test_save_and_get_note() {
        let provider = Box::new(MockProvider::new());
        let format = Arc::new(MockFormat);
        let mut repo = FileNotesRepository::new(provider, format);

        let note = Note {
            id: "note1".to_string(),
            title: "Test Note".to_string(),
            blocks: vec![Block::paragraph(vec![Inline::Text {
                text: "Hello World".into(),
            }])],
        };

        repo.save_note(&note).unwrap();

        let fetched = repo.get_note("note1").unwrap().unwrap();
        assert_eq!(fetched.id, "note1");
        assert_eq!(fetched.title, "Test Note");
        assert_eq!(fetched.blocks, note.blocks);
    }

    #[test]
    fn test_list_notes() {
        let provider = Box::new(MockProvider::new());
        let format = Arc::new(MockFormat);
        let mut repo = FileNotesRepository::new(provider, format);

        let note1 = Note {
            id: "n1".to_string(),
            title: "A".to_string(),
            blocks: vec![],
        };
        let note2 = Note {
            id: "n2".to_string(),
            title: "B".to_string(),
            blocks: vec![],
        };

        repo.save_note(&note1).unwrap();
        repo.save_note(&note2).unwrap();

        let notes = repo.list_notes().unwrap();
        let ids: Vec<_> = notes.iter().map(|n| n.id.clone()).collect();
        assert!(ids.contains(&"n1".to_string()));
        assert!(ids.contains(&"n2".to_string()));
    }

    #[test]
    fn test_delete_note() {
        let provider = Box::new(MockProvider::new());
        let formats = Arc::new(MockFormat);
        let mut repo = FileNotesRepository::new(provider, formats);

        let note = Note {
            id: "n1".to_string(),
            title: "Test".to_string(),
            blocks: vec![],
        };
        repo.save_note(&note).unwrap();

        assert!(repo.get_note("n1").unwrap().is_some());

        repo.delete_note("n1").unwrap();
        assert!(repo.get_note("n1").unwrap().is_none());
    }

    #[test]
    fn test_get_attachments() {
        let provider = Box::new(MockProvider::new());
        let format = Arc::new(MockFormat);
        let mut repo = FileNotesRepository::new(provider, format);

        let note = Note {
            id: "n1".to_string(),
            title: "Attachments".to_string(),
            blocks: vec![
                Block::paragraph(vec![Inline::Image {
                    alt_text: Some("img1".into()),
                    src: "file1.png".into(),
                }]),
                Block::image(Some("img2".into()), "file2.png".into()),
            ],
        };

        repo.save_note(&note).unwrap();

        let attachments = repo.get_attachments("n1").unwrap();

        let srcs: Vec<_> = attachments.iter().map(|a| a.name.clone()).collect();
        assert!(srcs.contains(&"file1.png".to_string()));
        assert!(srcs.contains(&"file2.png".to_string()));
    }
}
