use std::{cell::RefCell, sync::Arc};

use crate::{
    managers::{
        backlinks::BacklinkManager,
        tags::{GlobalTagManager, ScopedTagManager},
    },
    models::{LinkTarget, Note},
    repo::NotesRepository,
};

pub struct Vault {
    pub repo: Box<Arc<dyn NotesRepository>>,
    scoped_tags: ScopedTagManager,
    global_tags: Arc<GlobalTagManager>,
    backlinks: BacklinkManager,
}

impl Vault {
    pub fn new(repo: Arc<dyn NotesRepository>) -> Self {
        Self {
            repo: Box::new(repo),
            scoped_tags: ScopedTagManager::new(),
            global_tags: GlobalTagManager::new(),
            backlinks: BacklinkManager::new(),
        }
    }

    #[must_use]
    pub fn backlinks_for_note(&self, note_id: &str) -> Vec<String> {
        self.backlinks
            .backlinks_for(&LinkTarget::Note(note_id.to_string()))
    }

    #[must_use]
    pub fn backlinks_for_attachment(&self, attachment_id: &str) -> Vec<String> {
        self.backlinks
            .backlinks_for(&LinkTarget::Attachment(attachment_id.to_string()))
    }

    #[must_use]
    pub fn all_tags_for(&self, note: &Note) -> Vec<String> {
        let mut tags = Vec::new();
        if let Some(tagged) = self.scoped_tags.tag_index.get(&note.id) {
            tags.extend(tagged.iter().cloned());
        }
        tags.extend(self.global_tags.clone().get_tags_for(&note.id));
        tags
    }
}
