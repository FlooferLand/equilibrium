use std::path::{Path, PathBuf};

enum CommandData {
    Device(String),
    Include(PathBuf),
    Unknown { name: String, value: String }
}
struct CommandStatement {
    pub enabled: bool,
    pub data: CommandData
}

enum Statement {
    Command(CommandStatement),
    Comment(String)
}

/// EqualizerAPO config.txt file
pub struct RackFile {
    statements: Vec<Statement>
}
impl RackFile {
    pub fn get_custom_dir() -> PathBuf {
        Path::new("D:/Misc/EqualizerAPO/").to_path_buf()
    }
    pub fn get_root_path() -> PathBuf {
        Path::new("C:/Program Files/EqualizerAPO/config/config.txt").to_path_buf()
    }

    /// Loading config.txt
    pub fn load() -> anyhow::Result<RackFile> {
        let mut statements = Vec::new();

        let path = Self::get_root_path();
        let text = std::fs::read_to_string(path)?;
        for line in text.lines() {
            let is_comment = line.starts_with('#');
            let line = line.strip_prefix('#').unwrap_or(line);
            let line = line.trim().to_owned();
            let Some((name, value)) = line.split_once(':') else {
                statements.push(Statement::Comment(line));
                continue;
            };
            let name = name.trim().to_owned();
            let value = value.trim().to_owned();
            if name.is_empty() || value.is_empty() {
                statements.push(Statement::Comment(line));
                continue;
            }

            let command = CommandStatement {
                enabled: !is_comment,
                data: match name.as_str() {
                    "Device" => CommandData::Device(value),
                    "Include" => CommandData::Include(Path::new(&value).to_path_buf()),
                    _ => CommandData::Unknown { name, value }
                }
            };
            statements.push(Statement::Command(command));
        }
    
        Ok(RackFile { statements })
    }

    /// Saving to config.txt
    pub fn save(self) -> anyhow::Result<()> {
        let path = Self::get_root_path();
        
        let mut lines = Vec::new();
        for statement in self.statements {
            match statement {
                Statement::Command(command) => {
                    let line = match command.data {
                        CommandData::Device(device) => {
                            format!("Device: {device}")
                        }
                        CommandData::Include(path) => {
                            // println!("Pushing include '{}' (enabled={})", path.display(), &command.enabled);
                            format!("Include: {}", path.display())
                        }
                        CommandData::Unknown { name, value } => {
                            println!("Unknown config.txt command '{name}'");
                            format!("{name}: {value}")
                        },
                    };
                    if command.enabled {
                        lines.push(line);
                    } else {
                        lines.push(format!("# {line}"));
                    }
                },
                Statement::Comment(comment) => {
                    lines.push(format!("# {comment}"));
                },
            }
        }

        std::fs::write(path, lines.join("\n"))?;
        Ok(())
    }

    pub fn toggle_include(&mut self, request_path: PathBuf) -> anyhow::Result<bool> {
        let mut enabled = false;
        for statement in self.statements.iter_mut() {
            let Statement::Command(command) = statement else { continue };
            if let CommandData::Include(path) = &command.data {
                let path = path.to_owned();
                if path == request_path {
                    command.enabled = !command.enabled;
                    enabled = command.enabled;
                    break;
                }
            }
        }
        Ok(enabled)
    }
}
