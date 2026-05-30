// src/bevy_gui/resources/save_load.rs
//! Save/Load dialog state resource.

use bevy::prelude::*;

/// Information about a save file
#[derive(Debug, Clone)]
pub struct SaveFileInfo {
    pub filename: String,
    pub path: String,
    pub tick: u32,
    pub agent_count: usize,
    pub modified: String,
}

/// Save/Load dialog state resource
#[derive(Resource)]
pub struct SaveLoadState {
    /// Filename for saving (without extension)
    pub filename: String,
    /// Directory to save to / load from
    pub save_directory: String,
    /// List of available save files
    pub available_saves: Vec<SaveFileInfo>,
    /// Currently selected save file index
    pub selected_save: Option<usize>,
    /// Last error message
    pub last_error: Option<String>,
    /// Last success message
    pub last_success: Option<String>,
}

impl Default for SaveLoadState {
    fn default() -> Self {
        Self {
            filename: String::new(),
            save_directory: "./saves".to_string(),
            available_saves: Vec::new(),
            selected_save: None,
            last_error: None,
            last_success: None,
        }
    }
}

impl SaveLoadState {
    /// Clear all error/success messages
    pub fn clear_messages(&mut self) {
        self.last_error = None;
        self.last_success = None;
    }

    /// Set an error message
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.last_error = Some(message.into());
        self.last_success = None;
    }

    /// Set a success message
    pub fn set_success(&mut self, message: impl Into<String>) {
        self.last_success = Some(message.into());
        self.last_error = None;
    }

    /// Refresh the list of available save files
    pub fn refresh_saves(&mut self) {
        self.available_saves.clear();
        self.selected_save = None;

        let dir = if self.save_directory.is_empty() {
            "./saves"
        } else {
            &self.save_directory
        };

        // Ensure directory exists
        let _ = std::fs::create_dir_all(dir);

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "ebss") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        let modified = entry.metadata()
                            .and_then(|m| m.modified())
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|n| n.as_secs())
                                    .unwrap_or(0);
                                let secs = now.saturating_sub(d.as_secs());
                                let days = secs / 86400;
                                let hours = (secs % 86400) / 3600;
                                let mins = (secs % 3600) / 60;
                                if days > 0 {
                                    format!("{}d ago", days)
                                } else if hours > 0 {
                                    format!("{}h ago", hours)
                                } else if mins > 0 {
                                    format!("{}m ago", mins)
                                } else {
                                    "Just now".to_string()
                                }
                            })
                            .unwrap_or_else(|| "Unknown".to_string());

                        self.available_saves.push(SaveFileInfo {
                            filename: filename.to_string(),
                            path: path.to_string_lossy().to_string(),
                            tick: 0,
                            agent_count: 0,
                            modified,
                        });
                    }
                }
            }
        }

        // Sort by modification time (newest first would require storing the actual time)
        self.available_saves.sort_by(|a, b| a.filename.cmp(&b.filename));
    }

    /// Get the full save path for the current filename
    pub fn get_save_path(&self) -> String {
        let dir = if self.save_directory.is_empty() {
            "./saves"
        } else {
            &self.save_directory
        };
        format!("{}/{}.ebss", dir, self.filename)
    }

    /// Get the selected save file
    pub fn get_selected_save(&self) -> Option<&SaveFileInfo> {
        self.selected_save.and_then(|idx| self.available_saves.get(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_state_defaults() {
        let state = SaveLoadState::default();
        assert!(state.filename.is_empty());
        assert_eq!(state.save_directory, "./saves");
        assert!(state.available_saves.is_empty());
        assert!(state.selected_save.is_none());
    }

    #[test]
    fn test_save_load_messages() {
        let mut state = SaveLoadState::default();

        state.set_error("Test error");
        assert_eq!(state.last_error, Some("Test error".to_string()));
        assert!(state.last_success.is_none());

        state.set_success("Test success");
        assert_eq!(state.last_success, Some("Test success".to_string()));
        assert!(state.last_error.is_none());

        state.clear_messages();
        assert!(state.last_error.is_none());
        assert!(state.last_success.is_none());
    }

    #[test]
    fn test_get_save_path() {
        let mut state = SaveLoadState::default();
        state.filename = "my_save".to_string();

        assert_eq!(state.get_save_path(), "./saves/my_save.ebss");

        state.save_directory = "/tmp/ebss_saves".to_string();
        assert_eq!(state.get_save_path(), "/tmp/ebss_saves/my_save.ebss");
    }
}
