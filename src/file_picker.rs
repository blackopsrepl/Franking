/*! Filesystem picker for choosing an attachment path. */

use std::path::{Path, PathBuf};

/// What the picker was opened for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerPurpose {
    /// Choose a file to attach.
    Attach,
    /// Choose a directory to save an attachment into.
    SaveAttachment,
}

/// Browsable directory state.
#[derive(Debug, Clone)]
pub struct FilePickerState {
    /// Why the picker is open.
    pub purpose: PickerPurpose,
    pub dir: PathBuf,
    /// Directories first, then files, each sorted by name.
    pub entries: Vec<PathBuf>,
    pub index: usize,
    pub error: Option<String>,
}

impl FilePickerState {
    /// Open the picker for attaching a file.
    pub fn open(dir: Option<PathBuf>) -> Self {
        Self::open_with(PickerPurpose::Attach, dir)
    }

    /// Open the picker at `dir` (falling back to the home directory).
    pub fn open_with(purpose: PickerPurpose, dir: Option<PathBuf>) -> Self {
        let dir = dir
            .filter(|path| path.is_dir())
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let mut state = Self {
            purpose,
            dir,
            entries: Vec::new(),
            index: 0,
            error: None,
        };
        state.reload();
        state
    }

    /// Re-read the current directory.
    pub fn reload(&mut self) {
        match read_entries(&self.dir) {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
            }
            Err(error) => {
                self.entries.clear();
                self.error = Some(error);
            }
        }
        if self.index >= self.entries.len() {
            self.index = self.entries.len().saturating_sub(1);
        }
    }

    pub fn selected(&self) -> Option<&Path> {
        self.entries.get(self.index).map(PathBuf::as_path)
    }

    pub fn next(&mut self) {
        if !self.entries.is_empty() {
            self.index = (self.index + 1).min(self.entries.len() - 1);
        }
    }

    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    /// Descend into the selected directory.
    pub fn enter_directory(&mut self) {
        if let Some(path) = self.selected() {
            if path.is_dir() {
                self.dir = path.to_path_buf();
                self.index = 0;
                self.reload();
            }
        }
    }

    /// Move to the parent directory.
    pub fn parent_directory(&mut self) {
        if let Some(parent) = self.dir.parent() {
            self.dir = parent.to_path_buf();
            self.index = 0;
            self.reload();
        }
    }

    /// Resolve Enter: descend into a directory, or select a file.
    pub fn activate(&mut self) -> Option<PathBuf> {
        let path = self.selected()?.to_path_buf();
        if path.is_dir() {
            self.dir = path;
            self.index = 0;
            self.reload();
            None
        } else {
            Some(path)
        }
    }

    /// Render a display name for an entry.
    pub fn label(path: &Path) -> String {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        if path.is_dir() {
            format!("{name}/")
        } else {
            match std::fs::metadata(path) {
                Ok(metadata) => format!("{name}  ({} bytes)", metadata.len()),
                Err(_) => name,
            }
        }
    }
}

fn read_entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries = Vec::new();
    let read = std::fs::read_dir(dir).map_err(|error| format!("Cannot open directory: {error}"))?;
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        entries.push(entry.path());
    }
    entries.sort_by_key(|path| (path.is_file(), path.file_name().map(|n| n.to_owned())));
    Ok(entries)
}

#[cfg(test)]
mod tests;
