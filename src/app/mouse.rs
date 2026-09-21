/*! Mouse handling and hit-testing. */

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::keys::View;

use super::model::App;

impl App {
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        // Layout: header (row 0), body (rows 1..h-1), status (row h-1)
        // Body: sidebar cols 0..22, envelope list cols 23..
        // Message view: full width (no sidebar)
        let h = self.last_terminal_height;
        let row = mouse.row;
        let col = mouse.column;

        match mouse.kind {
            MouseEventKind::ScrollUp => match self.view {
                View::MessageView => self.scroll(-3),
                View::Help => self.scroll(-3),
                View::EnvelopeList => {
                    for _ in 0..3 {
                        self.move_selection(-1);
                    }
                }
                View::FolderList => self.move_selection(-1),
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.view {
                View::MessageView => self.scroll(3),
                View::Help => self.scroll(3),
                View::EnvelopeList => {
                    for _ in 0..3 {
                        self.move_selection(1);
                    }
                }
                View::FolderList => self.move_selection(1),
                _ => {}
            },
            MouseEventKind::Down(MouseButton::Left) => {
                if row == 0 || row >= h.saturating_sub(1) {
                    // Click on header or status bar -- ignore
                    return;
                }

                match self.view {
                    View::MessageView | View::Help | View::AccountList => {
                        // In overlays, clicks don't do navigation
                    }
                    _ => {
                        // Body area
                        let body_row = (row - 1) as usize; // offset past header

                        if col < 22 {
                            // Sidebar click
                            if body_row > 0 && body_row <= self.folders.len() {
                                // Account for border (row 0 of sidebar is border)
                                let folder_idx = body_row.saturating_sub(1);
                                if folder_idx < self.folders.len() {
                                    self.folder_index = folder_idx;
                                    self.view = View::FolderList;
                                    self.select_item();
                                }
                            }
                        } else {
                            // Envelope list click
                            // Account for border + header row (2 rows of table overhead)
                            if body_row >= 2 {
                                let env_idx = body_row.saturating_sub(2);
                                if env_idx < self.envelopes.len() {
                                    self.envelope_state.select(Some(env_idx));
                                    self.view = View::EnvelopeList;
                                }
                            }
                        }
                    }
                }
            }
            // Right-click to go back
            MouseEventKind::Down(MouseButton::Right) if self.view == View::MessageView => {
                self.go_back()
            }
            _ => {}
        }
    }

    // ── Key handling ────────────────────────────────────────────────
}
