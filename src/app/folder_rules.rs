/*! Explicit folder rules overlay and Move prompt opt-in state. */

use super::model::App;
use crate::db::folder_rules::{self as store, FolderRule};
use crate::keys::View;

#[derive(Default)]
pub struct FolderRulesState {
    pub open: bool,
    pub rules: Vec<FolderRule>,
    pub index: usize,
    pub pending_delete: Option<(String, String)>,
    /// Reset on every Move/Copy prompt; never inferred from an existing rule.
    pub also_future: bool,
}

#[cfg(test)]
#[path = "folder_rules_tests.rs"]
mod tests;

impl App {
    /// Consume overlay keys before normal key resolution, including Move's opt-in Tab.
    pub(crate) fn handle_folder_rules_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyEventKind};
        if key.kind == KeyEventKind::Release {
            return self.folder_rules.open;
        }
        if self.folder_rules.open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.close_folder_rules(),
                KeyCode::Down | KeyCode::Char('j') => self.folder_rules_next(),
                KeyCode::Up | KeyCode::Char('k') => self.folder_rules_prev(),
                KeyCode::Delete | KeyCode::Char('d') => self.delete_folder_rule(),
                _ => {
                    self.folder_rules.pending_delete = None;
                }
            }
            return true;
        }
        if self.view == View::MovePrompt && key.code == KeyCode::Tab {
            self.toggle_move_future_rule();
            return true;
        }
        false
    }

    pub(crate) fn open_folder_rules(&mut self) {
        self.folder_rules.pending_delete = None;
        self.reload_folder_rules();
        self.folder_rules.open = true;
    }

    fn reload_folder_rules(&mut self) {
        let result = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local database is unavailable"))
            .and_then(store::list);
        match result {
            Ok(rules) => {
                self.folder_rules.rules = rules;
                self.folder_rules.index = self
                    .folder_rules
                    .index
                    .min(self.folder_rules.rules.len().saturating_sub(1));
            }
            Err(error) => self.set_error(&format!("Could not load folder rules: {error}")),
        }
    }

    pub(crate) fn folder_rules_next(&mut self) {
        self.folder_rules.pending_delete = None;
        self.folder_rules.index =
            (self.folder_rules.index + 1).min(self.folder_rules.rules.len().saturating_sub(1));
    }

    pub(crate) fn folder_rules_prev(&mut self) {
        self.folder_rules.pending_delete = None;
        self.folder_rules.index = self.folder_rules.index.saturating_sub(1);
    }

    pub(crate) fn delete_folder_rule(&mut self) {
        let Some(rule) = self.folder_rules.rules.get(self.folder_rules.index) else {
            return;
        };
        let key = (rule.account.clone(), rule.sender.clone());
        if self.folder_rules.pending_delete.as_ref() != Some(&key) {
            self.folder_rules.pending_delete = Some(key);
            self.set_status(
                "Press d again to remove this future-mail rule (existing mail stays put).",
            );
            return;
        }
        let result = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local database is unavailable"))
            .and_then(|conn| store::delete(conn, &key.0, &key.1));
        self.folder_rules.pending_delete = None;
        match result {
            Ok(_) => {
                self.reload_folder_rules();
                self.set_status("Future-mail rule removed; existing messages were not moved.");
            }
            Err(error) => self.set_error(&format!("Could not remove folder rule: {error}")),
        }
    }

    pub(crate) fn close_folder_rules(&mut self) {
        self.folder_rules.pending_delete = None;
        self.folder_rules.open = false;
    }

    pub(crate) fn move_rule_sender(&self) -> Option<String> {
        if self.move_is_copy || self.target_ids().len() != 1 {
            return None;
        }
        self.selected_envelope().and_then(|e| e.sender.mailbox())
    }

    pub(crate) fn toggle_move_future_rule(&mut self) {
        if self.move_rule_sender().is_some() {
            self.folder_rules.also_future = !self.folder_rules.also_future;
        } else {
            self.set_status("Future-mail rules require moving one message with a sender mailbox.");
        }
    }

    pub(crate) fn save_move_future_rule(&self, target: &str) -> anyhow::Result<()> {
        let sender = self
            .move_rule_sender()
            .ok_or_else(|| anyhow::anyhow!("Select one message with a sender mailbox"))?;
        let account = self
            .selected_account()
            .ok_or_else(|| anyhow::anyhow!("Receiving account is unavailable"))?;
        if !self.folders.iter().any(|f| f.name == target) {
            anyhow::bail!("Choose an existing server folder for the future-mail rule");
        }
        let conn = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local database is unavailable"))?;
        store::save(conn, &account, &sender, target)
    }
}
