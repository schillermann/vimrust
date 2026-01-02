use std::{env, path::PathBuf};

use crate::{
    command_history_location::CommandHistoryLocation, command_history_root::CommandHistoryRoot,
};

pub struct CommandHistoryEnvironment;

impl CommandHistoryEnvironment {
    pub fn location(&self) -> CommandHistoryLocation {
        if cfg!(target_os = "windows") {
            return self.location_windows();
        }
        if cfg!(target_os = "macos") {
            return self.location_macos();
        }
        self.location_unix()
    }

    fn location_windows(&self) -> CommandHistoryLocation {
        let roaming = self.variable("APPDATA");
        let local = self.variable("LOCALAPPDATA");
        let base = match roaming {
            CommandHistoryEnvValue::Missing => match local {
                CommandHistoryEnvValue::Missing => CommandHistoryRoot::Missing,
                CommandHistoryEnvValue::Present { value } => {
                    CommandHistoryRoot::from(PathBuf::from(value))
                }
            },
            CommandHistoryEnvValue::Present { value } => {
                CommandHistoryRoot::from(PathBuf::from(value))
            }
        };
        base.location()
    }

    fn location_macos(&self) -> CommandHistoryLocation {
        let home = self.variable("HOME");
        let base = match home {
            CommandHistoryEnvValue::Missing => CommandHistoryRoot::Missing,
            CommandHistoryEnvValue::Present { value } => {
                let base = PathBuf::from(value)
                    .join("Library")
                    .join("Application Support");
                CommandHistoryRoot::from(base)
            }
        };
        base.location()
    }

    fn location_unix(&self) -> CommandHistoryLocation {
        let xdg_state = self.variable("XDG_STATE_HOME");
        let home = self.variable("HOME");
        let base = match xdg_state {
            CommandHistoryEnvValue::Present { value } => {
                CommandHistoryRoot::from(PathBuf::from(value))
            }
            CommandHistoryEnvValue::Missing => match home {
                CommandHistoryEnvValue::Missing => CommandHistoryRoot::Missing,
                CommandHistoryEnvValue::Present { value } => {
                    CommandHistoryRoot::from(PathBuf::from(value).join(".local").join("state"))
                }
            },
        };
        base.location()
    }

    fn variable(&self, key: &str) -> CommandHistoryEnvValue {
        match env::var(key) {
            Ok(value) => CommandHistoryEnvValue::Present { value },
            Err(_) => CommandHistoryEnvValue::Missing,
        }
    }
}

enum CommandHistoryEnvValue {
    Missing,
    Present { value: String },
}
