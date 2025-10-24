use std::collections::{HashMap, HashSet};

use crate::{
    formats::NoteFormat,
    models::{Attachment, LinkTarget, Note},
};

/// Manages a mapping of note → backlinks (who links to this note)
#[derive(Default)]
pub struct BacklinkManager {
    /// note_id -> set of note_ids that link to it
    backlinks: HashMap<LinkTarget, HashSet<String>>,
}

impl BacklinkManager {
    pub fn new() -> Self {
        Self {
            backlinks: HashMap::new(),
        }
    }

    /// Index all backlinks from the given notes using the provided format
    pub fn index_all(
        &mut self,
        notes: &[Note],
        attachments: &[Attachment],
        format: &dyn NoteFormat,
    ) {
        self.backlinks.clear();
        for note in notes {
            let targets = format.extract_links(note, attachments);
            for target in targets {
                self.backlinks
                    .entry(target)
                    .or_default()
                    .insert(note.id.clone());
            }
        }
    }

    /// Returns all note IDs that link to the given note ID
    pub fn backlinks_for(&self, target: &LinkTarget) -> Vec<String> {
        self.backlinks
            .get(target)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Optionally: get all outbound links from a specific note
    pub fn outbound_links(
        &self,
        note: &Note,
        attachments: &[Attachment],
        format: &dyn NoteFormat,
    ) -> Vec<LinkTarget> {
        format.extract_links(note, attachments)
    }
}

#[cfg(test)]
mod tests {

    use crate::{formats::markdown::MarkdownFormat, models::AttachmentType};

    use super::*;

    fn make_markdown_notes() -> (Vec<Note>, Vec<Attachment>) {
        let att1 = Attachment {
            id: "img1".into(),
            name: "Image 1".into(),
            kind: AttachmentType::Image,
        };
        let att2 = Attachment {
            id: "img2".into(),
            name: "Image 2".into(),
            kind: AttachmentType::Image,
        };

        let note_a_md = r#"
# Note A

This links to [Note B](b) and to an attachment ![Image](img1)
"#;

        let note_b_md = r#"
# Note B

This links to [Note C](c)
"#;

        let format = MarkdownFormat;

        let notes = vec![
            format.deserialize(note_a_md.as_bytes(), Some("a")),
            format.deserialize(note_b_md.as_bytes(), Some("b")),
        ];

        (notes, vec![att1, att2])
    }

    #[test]
    fn test_backlinks_with_markdown_format() {
        let format = MarkdownFormat;
        let (notes, attachments) = make_markdown_notes();
        let mut manager = BacklinkManager::new();

        manager.index_all(&notes, &attachments, &format);

        let backlinks_b = manager.backlinks_for(&LinkTarget::Note("b".into()));
        assert_eq!(backlinks_b, vec!["a"]);

        let backlinks_img1 = manager.backlinks_for(&LinkTarget::Attachment("img1".into()));
        assert_eq!(backlinks_img1, vec!["a"]);

        let outbound = manager.outbound_links(&notes[0], &attachments, &format);
        assert!(outbound.contains(&LinkTarget::Note("b".into())));
        assert!(outbound.contains(&LinkTarget::Attachment("img1".into())));
    }
}
