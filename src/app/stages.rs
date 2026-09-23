/*! Workflow stages: assign a conversation to a stage and filter by it. */

use crate::db::stages::{self as store, Stage};
use crate::keys::View;

use super::model::App;

/// Which stage-name prompt is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    New,
    Rename(i64),
}

/// State of the stage board.
#[derive(Default)]
pub struct StagesState {
    pub account: String,
    pub items: Vec<Stage>,
    pub index: usize,
    /// Anchors of the conversation the board was opened on.
    pub target: Vec<String>,
    pub input: String,
    pub naming: Option<NameKind>,
    pub pending_delete: Option<i64>,
}

impl App {
    /// Record each loaded conversation's stage and apply the active filter.
    pub(crate) fn load_stages_and_filter(&mut self, accounts: &[String]) {
        self.stage_of_anchor.clear();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        let mut stages_by_account: std::collections::HashMap<
            String,
            std::collections::HashMap<String, String>,
        > = std::collections::HashMap::new();
        for account in accounts {
            stages_by_account.insert(
                account.clone(),
                store::assignments_for_account(conn, account).unwrap_or_default(),
            );
        }
        for envelope in self.envelopes.iter() {
            let account = envelope
                .account
                .as_deref()
                .or(self.account_name.as_deref())
                .filter(|account| !account.is_empty());
            let Some(assignments) = account.and_then(|account| stages_by_account.get(account))
            else {
                continue;
            };
            let Some(anchors) = self.conversation_anchors.get(&envelope.id) else {
                continue;
            };
            for anchor in anchors {
                if let Some(stage) = assignments.get(anchor) {
                    self.stage_of_anchor.insert(anchor.clone(), stage.clone());
                }
            }
        }
        if let Some(filter) = self.stage_filter.clone() {
            let anchors = &self.conversation_anchors;
            let stage_of = &self.stage_of_anchor;
            self.envelopes.retain(|envelope| {
                anchors.get(&envelope.id).is_some_and(|list| {
                    list.iter()
                        .any(|anchor| stage_of.get(anchor) == Some(&filter))
                })
            });
        }
    }

    pub(crate) fn open_stages(&mut self) {
        let account = self.acct_owned().unwrap_or_default();
        self.stages.account = account.clone();
        self.stages.index = 0;
        self.stages.pending_delete = None;
        self.stages.target = self
            .selected_envelope()
            .and_then(|envelope| self.conversation_anchors.get(&envelope.id).cloned())
            .unwrap_or_default();
        self.reload_stages();
        self.view = View::StageBoard;
    }

    fn reload_stages(&mut self) {
        let account = self.stages.account.clone();
        self.stages.items = self
            .db
            .as_ref()
            .and_then(|conn| store::list(conn, &account).ok())
            .unwrap_or_default();
        self.stages.index = self
            .stages
            .index
            .min(self.stages.items.len().saturating_sub(1));
    }

    pub(crate) fn stages_next(&mut self) {
        if !self.stages.items.is_empty() {
            self.stages.index = (self.stages.index + 1).min(self.stages.items.len() - 1);
        }
    }

    pub(crate) fn stages_prev(&mut self) {
        self.stages.index = self.stages.index.saturating_sub(1);
    }

    pub(crate) fn close_stages(&mut self) {
        self.stages.pending_delete = None;
        self.view = View::EnvelopeList;
    }

    /// Move the conversation the board was opened on into the highlighted stage.
    pub(crate) fn stages_assign(&mut self) {
        let Some(stage) = self.stages.items.get(self.stages.index).cloned() else {
            return;
        };
        if self.stages.target.is_empty() {
            self.set_status("No conversation to stage.");
            return;
        }
        let (account, anchors) = (self.stages.account.clone(), self.stages.target.clone());
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::assign(conn, &account, &anchors, stage.id) {
            Ok(()) => {
                self.set_status(&format!("Staged as {}.", stage.name));
                self.close_stages();
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not stage the conversation: {error}")),
        }
    }

    /// Remove the conversation from whatever stage it is in.
    pub(crate) fn stages_unassign(&mut self) {
        if self.stages.target.is_empty() {
            self.set_status("No conversation to unstage.");
            return;
        }
        let (account, anchors) = (self.stages.account.clone(), self.stages.target.clone());
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::clear(conn, &account, &anchors) {
            Ok(()) => {
                self.set_status("Conversation unstaged.");
                self.close_stages();
                self.load_envelopes();
            }
            Err(error) => self.set_error(&format!("Could not unstage: {error}")),
        }
    }

    /// Focus the highlighted stage as a list filter, or clear it.
    pub(crate) fn stages_filter(&mut self) {
        let Some(stage) = self.stages.items.get(self.stages.index) else {
            return;
        };
        let name = stage.name.clone();
        if self.stage_filter.as_deref() == Some(name.as_str()) {
            self.stage_filter = None;
            self.set_status("Stage filter cleared.");
        } else {
            self.stage_filter = Some(name.clone());
            self.set_status(&format!("Showing stage {name}."));
        }
        self.close_stages();
        self.load_envelopes();
    }

    pub(crate) fn stages_begin_new(&mut self) {
        self.stages.input.clear();
        self.stages.naming = Some(NameKind::New);
        self.view = View::StageName;
    }

    pub(crate) fn stages_begin_rename(&mut self) {
        let Some(stage) = self.stages.items.get(self.stages.index) else {
            return;
        };
        self.stages.input = stage.name.clone();
        self.stages.naming = Some(NameKind::Rename(stage.id));
        self.view = View::StageName;
    }

    pub(crate) fn stage_name_input(&mut self, c: char) {
        self.stages.input.push(c);
    }

    pub(crate) fn stage_name_backspace(&mut self) {
        self.stages.input.pop();
    }

    pub(crate) fn cancel_stage_name(&mut self) {
        self.stages.naming = None;
        self.stages.input.clear();
        self.view = View::StageBoard;
    }

    pub(crate) fn submit_stage_name(&mut self) {
        let name = self.stages.input.trim().to_string();
        if name.is_empty() {
            self.set_error("A stage name is required.");
            return;
        }
        let kind = self.stages.naming;
        let account = self.stages.account.clone();
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let result = match kind {
            Some(NameKind::Rename(id)) => store::rename(conn, id, &name),
            _ => store::upsert(conn, &account, &name).map(|_| ()),
        };
        match result {
            Ok(()) => {
                self.stages.naming = None;
                self.stages.input.clear();
                self.reload_stages();
                self.view = View::StageBoard;
                self.set_status(&format!("Stage {name} saved."));
            }
            Err(error) => self.set_error(&format!("Could not save the stage: {error}")),
        }
    }

    pub(crate) fn stages_delete(&mut self) {
        let Some(stage) = self.stages.items.get(self.stages.index) else {
            return;
        };
        let id = stage.id;
        if self.stages.pending_delete != Some(id) {
            self.stages.pending_delete = Some(id);
            self.set_status("Press d again to delete this stage.");
            return;
        }
        self.stages.pending_delete = None;
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        match store::delete(conn, id) {
            Ok(_) => {
                self.reload_stages();
                self.set_status("Stage deleted.");
            }
            Err(error) => self.set_error(&format!("Could not delete the stage: {error}")),
        }
    }
}
