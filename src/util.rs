use std::path::PathBuf;

pub trait FileNameOrPathTrait {
    fn file_name_safe(self: &Self) -> String;
}

impl FileNameOrPathTrait for PathBuf {
    fn file_name_safe(self: &PathBuf) -> String {
        self.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or(self.display().to_string())
    }
}
