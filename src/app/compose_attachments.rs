/*! Compose attachment prompt handling. */

use crossterm::event::{KeyCode, KeyEvent};

use super::model::App;

impl App {
    /// Consume a key while the attach-path prompt is open. Returns false when
    /// the prompt is not active so normal compose handling proceeds.
    pub(crate) fn compose_handle_attach_input(&mut self, key: KeyEvent) -> bool {
        let Some(cs) = self.compose_state.as_mut() else {
            return false;
        };
        if cs.attach_input.is_none() {
            return false;
        }

        let mut submit = false;
        let mut cancel = false;
        let mut pick = false;
        {
            let input = cs.attach_input.as_mut().expect("checked above");
            match key.code {
                KeyCode::Enter => submit = true,
                KeyCode::Esc => cancel = true,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => input.push(c),
                KeyCode::Tab => pick = true,
                _ => {}
            }
        }

        let mut added = None;
        if submit {
            let path = cs
                .attach_input
                .take()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !path.is_empty() {
                cs.attachments.push(path.clone());
                cs.dirty = true;
                added = Some(path);
            }
        } else if cancel {
            cs.attach_input = None;
        }
        if pick {
            let start = cs.attach_input.clone();
            self.open_file_picker_at(start);
        }

        if let Some(path) = added {
            self.set_status(&format!("Attached {path}."));
        }
        true
    }
}
