//! Virtual file system for creating and manipulating files and folders in memory.

use std::{collections::HashMap, fs, io, path::Path};

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
    pub fn place(&self, path: &Path) -> io::Result<()> {
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

    #[cfg(feature = "zip")]
    /// Zip the folder and its contents into a zip archive.
    ///
    /// # Errors
    /// - If the zip archive cannot be written
    pub fn zip(&self, path: &Path) -> io::Result<()> {
        use io::Write;

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

    #[cfg(feature = "zip")]
    /// Zip the folder and its contents into a zip archive with the given comment.
    ///
    /// # Errors
    /// - If the zip archive cannot be written
    pub fn zip_with_comment<S>(&self, path: &Path, comment: S) -> io::Result<()>
    where
        S: Into<String>,
    {
        use io::Write;

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

impl TryFrom<&Path> for VFolder {
    type Error = io::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let mut root_vfolder = Self::new();
        let fs_root_folder = fs::read_dir(value)?;
        for dir_entry in fs_root_folder {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            let name = dir_entry.file_name().into_string().ok();
            if let Some(name) = name {
                if path.is_dir() {
                    root_vfolder.add_existing_folder(&name, Self::try_from(path.as_path())?);
                } else if path.is_file() {
                    let file = VFile::try_from(path.as_path())?;
                    root_vfolder.add_file(&name, file);
                } else {
                    unreachable!("Path is neither file nor directory");
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
impl Default for VFile {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl TryFrom<&Path> for VFile {
    type Error = io::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        let data = fs::read(value)?;
        Ok(Self::Binary(data))
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
