//! Transaction scaffolding for undo/redo.
//! A transaction bundles one or more commands and their inverse ops.

use crate::command::AppCommand;

#[derive(Debug, Default)]
pub struct Transaction {
    pub label: String,
    pub commands: Vec<AppCommand>,
}

impl Transaction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: AppCommand) {
        self.commands.push(cmd);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
