/*! Compose editor key handling and autocomplete. */

use crate::compose::FocusedField;

use super::model::App;

impl App {
    pub(crate) fn handle_editor_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // Clear send error on any key.
        if let Some(ref mut cs) = self.compose_state {
            cs.send_error = None;
        }
        let Some(ref mut cs) = self.compose_state else {
            return;
        };

        if cs.focused == FocusedField::From {
            match key.code {
                KeyCode::Char(' ') | KeyCode::Right | KeyCode::Enter => {
                    cs.cycle_from_next();
                }
                KeyCode::Left => {
                    cs.cycle_from_prev();
                }
                _ => {}
            }
            return;
        }

        if cs.autocomplete.is_some() && cs.is_header_focused() {
            match key.code {
                KeyCode::Down => {
                    if let Some(ref mut ac) = cs.autocomplete {
                        ac.move_down();
                    }
                    return;
                }
                KeyCode::Up => {
                    if let Some(ref mut ac) = cs.autocomplete {
                        ac.move_up();
                    }
                    return;
                }
                KeyCode::Enter | KeyCode::Tab => {
                    // Accept selected suggestion
                    let accepted = cs.autocomplete.as_ref().and_then(|ac| ac.current());
                    if let Some(suggestion) = accepted {
                        if let Some(f) = cs.focused_line_field_mut() {
                            let prefix = f
                                .rfind(',')
                                .map(|i| f[..=i].to_string() + " ")
                                .unwrap_or_default();
                            *f = format!("{}{}", prefix, suggestion);
                            cs.dirty = true;
                        }
                    }
                    cs.autocomplete = None;
                    if key.code == KeyCode::Tab {
                        cs.focused = cs.focused.next();
                    }
                    return;
                }
                KeyCode::Esc => {
                    cs.autocomplete = None;
                    return;
                }
                _ => {} // fall through
            }
        }

        if matches!(
            cs.focused,
            FocusedField::To | FocusedField::Cc | FocusedField::Bcc | FocusedField::Subject
        ) {
            let is_address = matches!(
                cs.focused,
                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
            );
            let mut modified = false;
            let mut move_focus = false;
            if let Some(field) = cs.focused_line_field_mut() {
                match key.code {
                    KeyCode::Char(c)
                        if !key.modifiers.intersects(
                            crossterm::event::KeyModifiers::CONTROL
                                | crossterm::event::KeyModifiers::ALT,
                        ) =>
                    {
                        field.push(c);
                        modified = true;
                    }
                    KeyCode::Backspace => {
                        modified = field.pop().is_some();
                    }
                    KeyCode::Enter => {
                        move_focus = true;
                    }
                    _ => {}
                }
            }
            if modified {
                cs.dirty = true;
            }
            if move_focus {
                cs.autocomplete = None;
                cs.focused = cs.focused.next();
            }
            let _ = cs;
            if is_address && modified {
                self.update_autocomplete();
            }
            return;
        }

        if cs.focused == FocusedField::Body {
            let result = cs.body.handle_key(key);
            if result.text_modified {
                cs.dirty = true;
            }
        }
    }

    // ── Autocomplete ─────────────────────────────────────────────────

    /// Update the autocomplete popup for the currently focused address field.
    /// Called after every character typed in To/Cc/Bcc.  Results come from
    /// a synchronous search of the encrypted contacts DB.
    pub(crate) fn update_autocomplete(&mut self) {
        // Extract the current query token (text after the last comma).
        let query = {
            let Some(ref cs) = self.compose_state else {
                return;
            };
            let field_value = match cs.focused {
                FocusedField::To => &cs.to,
                FocusedField::Cc => &cs.cc,
                FocusedField::Bcc => &cs.bcc,
                _ => return,
            };
            // Get the last token after a comma (in case of multiple addresses)
            let token = field_value
                .rfind(',')
                .map(|i| field_value[i + 1..].trim())
                .unwrap_or(field_value.as_str());
            token.to_string()
        };

        // Require at least 2 characters to trigger autocomplete.
        if query.len() < 2 {
            if let Some(ref mut cs) = self.compose_state {
                cs.autocomplete = None;
            }
            return;
        }

        // Run search synchronously against the DB.
        let results: Vec<(Option<String>, String)> = if let Some(ref conn) = self.db {
            crate::contacts::search(conn, &query, 8)
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.name, c.email))
                .collect()
        } else {
            vec![]
        };

        if let Some(ref mut cs) = self.compose_state {
            if results.is_empty() {
                cs.autocomplete = None;
            } else {
                let field = cs.focused;
                cs.autocomplete = Some(crate::compose::AutocompleteState::new(field, results));
            }
        }
    }

    // ── Contacts ─────────────────────────────────────────────────────
}
