use std::fmt::Debug;

use uniffi::trait_interface;

use crate::models::Note;

pub mod file;
pub mod memory;

pub type RepoResult<T> = Result<T, Box<dyn core::error::Error>>;

#[trait_interface]
pub trait NotesRepository: Send + Sync + Debug {
    /// List all notes
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the repository fails
    fn list_notes(&self) -> RepoResult<Vec<Note>>;

    /// Get a note by ID
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the repository fails
    fn get_note(&self, id: &str) -> RepoResult<Option<Note>>;

    /// Save a note
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the repository fails
    fn save_note(&mut self, note: &Note) -> RepoResult<()>;

    /// Delete a note
    ///
    /// # Errors
    ///
    /// Returns an error if deleting from the repository fails
    fn delete_note(&mut self, id: &str) -> RepoResult<()>;
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use super::*;
    use crate::{
        formats::markdown::MarkdownFormat,
        models::{Block, Inline, Note},
        repo::{
            file::{FileNotesRepository, FileProvider},
            memory::MemoryNotesRepository,
        },
    };

    #[derive(Debug)]
    pub struct MockFileProvider {
        files: BTreeMap<String, Vec<u8>>,
    }

    impl MockFileProvider {
        pub fn new() -> Self {
            Self {
                files: BTreeMap::new(),
            }
        }
    }

    impl FileProvider for MockFileProvider {
        fn read(&self, id: &str) -> Option<Vec<u8>> {
            self.files.get(id).cloned()
        }

        fn write(&mut self, id: &str, data: &[u8]) -> bool {
            self.files.insert(id.to_string(), data.to_vec());
            true
        }

        fn delete(&mut self, id: &str) -> bool {
            self.files.remove(id).is_some()
        }

        fn list(&self) -> Vec<String> {
            self.files.keys().cloned().collect()
        }
    }

    #[test]
    fn memory_repo_basic_operations() {
        let format = Arc::new(MarkdownFormat);
        let mut repo = MemoryNotesRepository::new(format);

        let note = Note {
            id: "1".to_string(),
            title: "Hello".to_string(),
            blocks: vec![Block::paragraph(vec![Inline::Text {
                text: "World".to_string(),
            }])],
        };

        repo.save_note(&note).unwrap();

        let loaded = repo.get_note("1").unwrap().unwrap();
        assert_eq!(loaded.title, "Hello");

        let notes = repo.list_notes().unwrap();
        assert_eq!(notes.len(), 1);

        repo.delete_note("1").unwrap();
        assert!(repo.get_note("1").unwrap().is_none());
    }

    #[test]
    fn file_repo_basic_operations() {
        let provider = Box::new(MockFileProvider::new());
        let format = Arc::new(MarkdownFormat);
        let mut repo = FileNotesRepository::new(provider, format);

        let note = Note {
            id: "note1".to_string(),
            title: "File Note".to_string(),
            blocks: vec![Block::paragraph(vec![Inline::Text {
                text: "Content".to_string(),
            }])],
        };

        repo.save_note(&note).unwrap();

        let loaded = repo.get_note("note1").unwrap().unwrap();
        assert_eq!(loaded.title, "File Note");

        let notes = repo.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, "note1");

        repo.delete_note("note1").unwrap();
        assert!(repo.get_note("note1").unwrap().is_none());
    }
}
