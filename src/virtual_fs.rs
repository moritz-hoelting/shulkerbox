//! Virtual file system for creating and manipulating files and folders in memory.

use std::{collections::HashMap, path::Path};

#[cfg(feature = "zip")]
use zip::ZipWriter;

/// Folder representation in virtual file system
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VFolder {
    folders: HashMap<String, VFolder>,
    files: HashMap<String, VFile>,
}
impl VFolder {
    /// Create a new, empty virtual folder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            folders: HashMap::new(),
            files: HashMap::new(),
        }
    }

    /// Get all direct subfolders in the folder.
    #[must_use]
    pub fn get_folders(&self) -> &HashMap<String, Self> {
        &self.folders
    }
    /// Get all direct files in the folder.
    #[must_use]
    pub fn get_files(&self) -> &HashMap<String, VFile> {
        &self.files
    }

    /// Recursively add a new folder to the folder.
    pub fn add_folder(&mut self, path: &str) {
        self.add_existing_folder(path, Self::new());
    }
    /// Recursively add an existing folder to the folder.
    pub fn add_existing_folder(&mut self, path: &str, folder: Self) {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            // if the folder already exists, add the subfolder to it
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_folder(tail);
            } else {
                let mut new_folder = Self::new();
                new_folder.add_folder(tail);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.folders.insert(path.to_string(), folder);
        }
    }
    /// Recursively add a new file to the folder.
    pub fn add_file(&mut self, path: &str, file: VFile) {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            // if the folder already exists, add the file to it
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_file(tail, file);
            } else {
                let mut new_folder = Self::new();
                new_folder.add_file(tail, file);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.files.insert(path.to_string(), file);
        }
    }

    /// Recursively get a subfolder by path.
    #[must_use]
    pub fn get_folder(&self, path: &str) -> Option<&Self> {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_folder(tail)
        } else {
            self.folders.get(path)
        }
    }
    /// Recursively get a mutable subfolder by path.
    pub fn get_folder_mut(&mut self, path: &str) -> Option<&mut Self> {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_folder_mut(tail)
        } else {
            self.folders.get_mut(path)
        }
    }
    /// Recursively get a file by path.
    #[must_use]
    pub fn get_file(&self, path: &str) -> Option<&VFile> {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_file(tail)
        } else {
            self.files.get(path)
        }
    }
    /// Recursively get a mutable file by path.
    pub fn get_file_mut(&mut self, path: &str) -> Option<&mut VFile> {
        // extract first folder name and the rest of the path
        let (head, tail) = path
            .split_once('/')
            .map_or((path, None), |(h, t)| (h, (!t.is_empty()).then_some(t)));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_file_mut(tail)
        } else {
            self.files.get_mut(path)
        }
    }

    /// Place the folder and its contents on the file system.
    ///
    /// # Errors
    /// - If the folder cannot be written
    #[cfg(feature = "fs_access")]
    pub fn place(&self, path: &Path) -> std::io::Result<()> {
        use std::fs;

        fs::create_dir_all(path)?;
        // place each subfolder recursively
        for (name, folder) in &self.folders {
            folder.place(&path.join(name))?;
        }
        // create each file
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

    /// Zip the folder and its contents into a zip archive.
    ///
    /// # Errors
    /// - If the zip archive cannot be written
    #[cfg(all(feature = "fs_access", feature = "zip"))]
    pub fn zip(&self, path: &Path) -> std::io::Result<()> {
        use std::{fs, io::Write};

        // open target file
        let file = fs::File::create(path)?;
        let mut writer = ZipWriter::new(file);
        let virtual_files = self.flatten();

        // write each file to the zip archive
        for (path, file) in virtual_files {
            writer.start_file(path, zip::write::SimpleFileOptions::default())?;
            match file {
                VFile::Text(text) => {
                    writer.write_all(text.as_bytes())?;
                }
                VFile::Binary(data) => {
                    writer.write_all(data)?;
                }
            }
        }

        writer.set_comment("Data pack created with Shulkerbox");

        writer.finish()?;

        Ok(())
    }

    /// Zip the folder and its contents into a zip archive with the given comment.
    ///
    /// # Errors
    /// - If the zip archive cannot be written
    #[cfg(all(feature = "fs_access", feature = "zip"))]
    pub fn zip_with_comment<S>(&self, path: &Path, comment: S) -> std::io::Result<()>
    where
        S: Into<String>,
    {
        use std::{fs, io::Write};

        // open target file
        let file = fs::File::create(path)?;
        let mut writer = ZipWriter::new(file);
        let virtual_files = self.flatten();

        // write each file to the zip archive
        for (path, file) in virtual_files {
            writer.start_file(path, zip::write::SimpleFileOptions::default())?;
            match file {
                VFile::Text(text) => {
                    writer.write_all(text.as_bytes())?;
                }
                VFile::Binary(data) => {
                    writer.write_all(data)?;
                }
            }
        }

        let comment: String = comment.into();
        if !comment.is_empty() {
            writer.set_comment(comment);
        }

        writer.finish()?;

        Ok(())
    }

    /// Flatten the folder and its contents into a list of files with full paths.
    #[must_use]
    pub fn flatten(&self) -> Vec<(String, &VFile)> {
        let mut files = self
            .files
            .iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Vec<_>>();

        for (name, folder) in &self.folders {
            let sub_files = folder
                .flatten()
                .into_iter()
                .map(|(path, file)| (format!("{name}/{path}"), file))
                .collect::<Vec<_>>();
            files.extend(sub_files);
        }

        files
    }

    /// Recursively merge another folder into this folder.
    /// Returns a list of paths that were replaced by other.
    pub fn merge(&mut self, other: Self) -> Vec<String> {
        self._merge(other, "")
    }

    fn _merge(&mut self, other: Self, prefix: &str) -> Vec<String> {
        let mut replaced = Vec::new();
        for (name, folder) in other.folders {
            if let Some(existing_folder) = self.folders.get_mut(&name) {
                let replaced_folder = existing_folder._merge(folder, &format!("{prefix}{name}/"));
                replaced.extend(replaced_folder);
            } else {
                self.folders.insert(name, folder);
            }
        }
        for (name, file) in other.files {
            let replaced_file = self.files.insert(name.clone(), file);
            if replaced_file.is_some() {
                replaced.push(format!("{prefix}{name}"));
            }
        }

        replaced
    }
}

#[cfg(feature = "fs_access")]
impl TryFrom<&Path> for VFolder {
    type Error = std::io::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        use std::{fs, io};

        let mut root_vfolder = Self::new();
        let fs_root_folder = fs::read_dir(value)?;
        for dir_entry in fs_root_folder {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            let name = dir_entry.file_name().into_string().ok();
            if let Some(name) = name {
                if path.is_dir() {
                    root_vfolder.add_existing_folder(&name, Self::try_from(path.as_path())?);
                } else {
                    let file = VFile::try_from(path.as_path())?;
                    root_vfolder.add_file(&name, file);
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid file name",
                ));
            }
        }

        Ok(root_vfolder)
    }
}

/// File representation in virtual file system
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VFile {
    /// Text file
    Text(String),
    /// Binary file
    Binary(Vec<u8>),
}

impl From<String> for VFile {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<&str> for VFile {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}
impl From<Vec<u8>> for VFile {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(value)
    }
}
impl From<&[u8]> for VFile {
    fn from(value: &[u8]) -> Self {
        Self::Binary(value.to_vec())
    }
}

#[cfg(feature = "fs_access")]
impl TryFrom<&Path> for VFile {
    type Error = std::io::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let data = std::fs::read(value)?;
        Ok(Self::Binary(data))
    }
}
impl Default for VFile {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl VFile {
    /// Get the text content of the file.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Binary(_) => None,
        }
    }
    /// Get the binary content of the file.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Binary(data) => data,
            Self::Text(text) => text.as_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

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

        v_folder.add_file("bar/foo.bin", VFile::Binary(vec![1, 2, 3, 4]));

        assert_eq!(v_folder.get_files().len(), 1);
        assert_eq!(v_folder.get_folders().len(), 1);
        assert!(v_folder.get_file("bar/baz.txt").is_some());
        assert!(v_folder
            .get_folder("bar")
            .expect("folder not found")
            .get_file("baz.txt")
            .is_some());

        let temp = tempfile::tempdir().expect("failed to create temp dir");
        v_folder.place(temp.path()).expect("failed to place folder");

        assert_eq!(
            fs::read_to_string(temp.path().join("foo.txt")).expect("failed to read file"),
            "foo"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("bar/baz.txt")).expect("failed to read file"),
            "baz"
        );
        assert_eq!(
            fs::read(temp.path().join("bar/foo.bin")).expect("failed to read file"),
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn test_flatten() {
        let mut v_folder = VFolder::new();
        v_folder.add_file("a.txt", VFile::from("a"));
        v_folder.add_file("a/b.txt", VFile::from("b"));
        v_folder.add_file("a/b/c.txt", VFile::from("c"));

        let flattened = v_folder.flatten();
        assert_eq!(flattened.len(), 3);
        assert!(flattened.iter().any(|(path, _)| path == "a.txt"));
        assert!(flattened.iter().any(|(path, _)| path == "a/b.txt"));
        assert!(flattened.iter().any(|(path, _)| path == "a/b/c.txt"));
    }

    #[test]
    fn test_merge() {
        let mut first = VFolder::new();
        first.add_file("a.txt", VFile::from("a"));
        first.add_file("a/b.txt", VFile::from("b"));

        let mut second = VFolder::new();
        second.add_file("a.txt", VFile::from("a2"));
        second.add_file("c.txt", VFile::from("c"));
        second.add_file("c/d.txt", VFile::from("d"));
        second.add_file("a/e.txt", VFile::from("e"));

        let replaced = first.merge(second);
        assert_eq!(replaced.len(), 1);

        assert!(first.get_file("a.txt").is_some());
        assert!(first.get_file("a/b.txt").is_some());
        assert!(first.get_file("c.txt").is_some());
        assert!(first.get_file("c/d.txt").is_some());
        assert!(first.get_file("a/e.txt").is_some());
    }

    #[test]
    fn test_try_from() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        fs::create_dir_all(temp_dir.path().join("bar")).expect("failed to create dir");
        fs::write(temp_dir.path().join("foo.txt"), "foo").expect("failed to write file");
        fs::write(temp_dir.path().join("bar/baz.txt"), "baz").expect("failed to write file");

        let v_folder = VFolder::try_from(temp_dir.path()).expect("failed to convert");
        assert_eq!(v_folder.get_files().len(), 1);
        assert_eq!(v_folder.get_folders().len(), 1);
        if let VFile::Binary(data) = v_folder.get_file("foo.txt").expect("file not found") {
            assert_eq!(data, b"foo");
        } else {
            panic!("File is not binary");
        }
        if let VFile::Binary(data) = v_folder.get_file("bar/baz.txt").expect("file not found") {
            assert_eq!(data, b"baz");
        } else {
            panic!("File is not binary");
        }
    }
}
