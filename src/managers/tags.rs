use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use uniffi::export;

use crate::models::Note;

/// Global tags assigned externally
#[derive(Debug, Default, Clone, uniffi::Object)]
pub struct GlobalTagManager {
    global_tags: RefCell<HashMap<String, Vec<String>>>,
}

unsafe impl Send for GlobalTagManager {}
unsafe impl Sync for GlobalTagManager {}

#[export]
impl GlobalTagManager {
    #[must_use]
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            global_tags: RefCell::new(HashMap::new()),
        })
    }

    #[uniffi::method]
    pub fn assign_tag(self: Arc<Self>, note_id: &str, tag: String) {
        self.global_tags
            .borrow_mut()
            .entry(note_id.to_string())
            .or_default()
            .push(tag);
    }

    #[must_use]
    #[uniffi::method]
    pub fn get_tags_for(self: Arc<Self>, note_id: &str) -> Vec<String> {
        self.global_tags
            .borrow()
            .get(note_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// Tags extracted from content (if supported by format)
pub struct ScopedTagManager {
    pub tag_index: HashMap<String, HashSet<String>>,
}

impl Default for ScopedTagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopedTagManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tag_index: HashMap::new(),
        }
    }

    pub fn index_note(&mut self, note: &Note, extract_tags: impl Fn(&Note) -> Vec<String>) {
        for tag in extract_tags(note) {
            self.tag_index
                .entry(tag)
                .or_default()
                .insert(note.id.clone());
        }
    }

    #[must_use]
    pub fn notes_with_tag(&self, tag: &str) -> Vec<String> {
        self.tag_index
            .get(tag)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tag_tests {

    use crate::{
        managers::tags::{GlobalTagManager, ScopedTagManager},
        models::Note,
    };

    #[test]
    fn test_tags() {
        let notes = vec![
            Note {
                id: "a".into(),
                title: "A".into(),
                blocks: vec![],
            },
            Note {
                id: "b".into(),
                title: "B".into(),
                blocks: vec![],
            },
        ];

        let mut scoped = ScopedTagManager::new();
        for note in &notes {
            scoped.index_note(note, |_note| vec!["scoped".into()]);
        }

        let global = GlobalTagManager::new();
        global.clone().assign_tag("a", "global".into());

        assert_eq!(scoped.notes_with_tag("scoped").len(), 2);
        assert_eq!(global.get_tags_for("a"), vec!["global"]);
    }
}
