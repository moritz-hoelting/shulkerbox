use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct VFolder {
    folders: HashMap<String, VFolder>,
    files: HashMap<String, VFile>,
}
impl VFolder {
    pub fn new() -> VFolder {
        VFolder {
            folders: HashMap::new(),
            files: HashMap::new(),
        }
    }

    pub fn get_folders(&self) -> &HashMap<String, VFolder> {
        &self.folders
    }
    pub fn get_files(&self) -> &HashMap<String, VFile> {
        &self.files
    }

    pub fn add_folder(&mut self, name: &str) {
        self.add_existing_folder(name, VFolder::new());
    }
    pub fn add_existing_folder(&mut self, name: &str, folder: VFolder) {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_folder(tail);
            } else {
                let mut new_folder = VFolder::new();
                new_folder.add_folder(tail);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.folders.insert(name.to_string(), folder);
        }
    }
    pub fn add_file(&mut self, name: &str, file: VFile) {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            if let Some(subfolder) = self.get_folder_mut(head) {
                subfolder.add_file(tail, file);
            } else {
                let mut new_folder = VFolder::new();
                new_folder.add_file(tail, file);
                self.add_existing_folder(head, new_folder);
            }
        } else {
            self.files.insert(name.to_string(), file);
        }
    }

    pub fn get_folder(&self, name: &str) -> Option<&VFolder> {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_folder(tail)
        } else {
            self.folders.get(name)
        }
    }
    pub fn get_folder_mut(&mut self, name: &str) -> Option<&mut VFolder> {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_folder_mut(tail)
        } else {
            self.folders.get_mut(name)
        }
    }
    pub fn get_file(&self, name: &str) -> Option<&VFile> {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            self.folders.get(head)?.get_file(tail)
        } else {
            self.files.get(name)
        }
    }
    pub fn get_file_mut(&mut self, name: &str) -> Option<&mut VFile> {
        let (head, tail) = name
            .split_once('/')
            .map(|(h, t)| (h, (!t.is_empty()).then_some(t)))
            .unwrap_or((name, None));
        if let Some(tail) = tail {
            self.folders.get_mut(head)?.get_file_mut(tail)
        } else {
            self.files.get_mut(name)
        }
    }
}

#[derive(Debug, Clone)]
pub enum VFile {
    Text(String),
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
