use std::{
    collections::HashSet,
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use note::{
    formats::markdown::MarkdownFormat,
    repo::file::{FileNotesRepository, FileProvider},
    vault::Vault,
};

#[derive(Debug)]
struct LocalFileProvider {
    base_path: PathBuf,
}

impl LocalFileProvider {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let path = base_path.as_ref();
        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Self { base_path: abs }
    }

    fn resolve_path(&self, relative: &str) -> PathBuf {
        let path = Path::new(relative);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_path.join(path)
        }
    }

    fn list_recursive(&self, dir: &Path, files: &mut Vec<String>, visited: &mut HashSet<PathBuf>) {
        let canonical = match fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => return,
        };

        if !visited.insert(canonical.clone()) {
            println!("Skipping already visited: {:?}", canonical);
            return;
        }

        if let Ok(entries) = fs::read_dir(&canonical) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.to_str() {
                        files.push(name.to_string());
                    }
                } else if path.is_dir() {
                    self.list_recursive(&path, files, visited);
                }
            }
        }
    }
}

impl FileProvider for LocalFileProvider {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        let full_path = self.resolve_path(path);
        let data = std::fs::read(full_path).ok()?;
        Some(data)
    }

    fn write(&mut self, path: &str, data: &[u8]) -> bool {
        std::fs::write(self.resolve_path(path), data).is_ok()
    }

    fn delete(&mut self, path: &str) -> bool {
        std::fs::remove_file(self.resolve_path(path)).is_ok()
    }

    fn list(&self) -> Vec<String> {
        println!("Listing files in {:?}", self.base_path);
        let mut files = Vec::new();
        let mut visited = HashSet::new();
        self.list_recursive(&self.base_path, &mut files, &mut visited);
        println!("Total files: {}", files.len());
        files
    }
}

fn main() {
    let file_provider = Box::new(LocalFileProvider::new("examples/my_vault"));
    let markdown_format = Arc::new(MarkdownFormat);

    let markdown_repo = Arc::new(FileNotesRepository::new(file_provider, markdown_format));
    let mut writer = BufWriter::new(std::io::stdout());

    let vault = Vault::new(markdown_repo);

    if let Some(note) = vault
        .repo
        .get_note("Getting started/Create a vault.md")
        .expect("Failed to list notes")
    {
        let backlinks = vault.backlinks_for_note(&note.id);
        let tags = vault.all_tags_for(&note);

        writeln!(&mut writer, "Note: {}", note.title).unwrap();
        writeln!(&mut writer, "Backlinks: {:?}", backlinks).unwrap();
        writeln!(&mut writer, "Tags: {:?}", tags).unwrap();
        writeln!(&mut writer, "---").unwrap();

        for block in &note.blocks {
            writeln!(&mut writer, "{:?}", block).unwrap();
        }
    }
}
