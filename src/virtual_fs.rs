//! Virtual file system for creating and manipulating files and folders in memory.

use std::{collections::HashMap, fs, io, path::Path};

/// Folder representation in virtual file system
#[derive(Debug, Default, Clone)]
pub struct VFolder {
    folders: HashMap<String, VFolder>,
    files: HashMap<String, VFile>,
}
impl VFolder {
    /// Create a new, empty virtual folder.
    pub fn new() -> VFolder {
        VFolder {
            folders: HashMap::new(),
            files: HashMap::new(),
        }
    }

    /// Get all direct subfolders in the folder.
    pub fn get_folders(&self) -> &HashMap<String, VFolder> {
        &self.folders
    }
    /// Get all direct files in the folder.
    pub fn get_files(&self) -> &HashMap<String, VFile> {
        &self.files
    }

    /// Recursively add a new folder to the folder.
    pub fn add_folder(&mut self, path: &str) {
        self.add_existing_folder(path, VFolder::new());
    }
    /// Recursively add an existing folder to the folder.
    pub fn add_existing_folder(&mut self, path: &str, folder: VFolder) {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_folder(tail);
            } else {
                let mut new_folder = VFolder::new();
                new_folder.add_folder(tail);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.folders.insert(path.to_string(), folder);
        }
    }
    /// Recursively add a new file to the folder.
    pub fn add_file(&mut self, path: &str, file: VFile) {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_file(tail, file);
            } else {
                let mut new_folder = VFolder::new();
                new_folder.add_file(tail, file);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.files.insert(path.to_string(), file);
        }
    }

    /// Recursively get a subfolder by path.
    pub fn get_folder(&self, path: &str) -> Option<&VFolder> {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_folder(tail)
        } else {
            self.folders.get(path)
        }
    }
    /// Recursively get a mutable subfolder by path.
    pub fn get_folder_mut(&mut self, path: &str) -> Option<&mut VFolder> {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_folder_mut(tail)
        } else {
            self.folders.get_mut(path)
        }
    }
    /// Recursively get a file by path.
    pub fn get_file(&self, path: &str) -> Option<&VFile> {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_file(tail)
        } else {
            self.files.get(path)
        }
    }
    /// Recursively get a mutable file by path.
    pub fn get_file_mut(&mut self, path: &str) -> Option<&mut VFile> {
        let (head, tail) = path
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((path, None));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_file_mut(tail)
        } else {
            self.files.get_mut(path)
        }
    }

    /// Place the folder and its contents on the file system.
    pub fn place(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)?;
        for (name, folder) in &self.folders {
            folder.place(&path.join(name))?;
        }
        for (name, file) in &self.files {
            match file {
                VFile::Text(text) => {
                    fs::write(path.join(name), text)?;
                }
                VFile::Binary(data) => {
                    fs::write(path.join(name), data)?;
                }
            }
        }
        Ok(())
    }
}

/// File representation in virtual file system
#[derive(Debug, Clone)]
pub enum VFile {
    /// Text file
    Text(String),
    /// Binary file
    Binary(Vec<u8>),
}

impl From<String> for VFile {
    fn from(value: String) -> Self {
        VFile::Text(value)
    }
}
impl From<&str> for VFile {
    fn from(value: &str) -> Self {
        VFile::Text(value.to_string())
    }
}
impl Default for VFile {
    fn default() -> Self {
        VFile::Text(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfolder() {
        let mut v_folder = VFolder::new();
        let v_file_1 = VFile::from("foo");
        v_folder.add_file("foo.txt", v_file_1);

        assert_eq!(v_folder.get_files().len(), 1);
        assert_eq!(v_folder.get_folders().len(), 0);

        let v_file_2 = VFile::from("baz");
        v_folder.add_file("bar/baz.txt", v_file_2);

        assert_eq!(v_folder.get_files().len(), 1);
        assert_eq!(v_folder.get_folders().len(), 1);
        assert!(v_folder.get_file("bar/baz.txt").is_some());
        assert!(v_folder
            .get_folder("bar")
            .expect("folder not found")
            .get_file("baz.txt")
            .is_some());
    }
}
