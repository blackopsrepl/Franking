/*! Adding a message's calendar invitation to Planner123. */

use crate::mail::planner123::Planner123;

use super::model::App;

impl App {
    /// The iCalendar payload of the loaded message's invitation, if any.
    fn message_invitation_ics(&self) -> Option<Vec<u8>> {
        let message = self.message_content.as_ref()?;
        for part in &message.parts {
            let mut ics = None;
            part.walk(&mut |part| {
                if ics.is_none() && part.content_type.eq_ignore_ascii_case("text/calendar") {
                    ics = part.text().map(|text| text.as_bytes().to_vec());
                }
            });
            if ics.is_some() {
                return ics;
            }
        }
        None
    }

    /// Hand the loaded message's invitation to Planner123.
    pub(crate) fn add_to_planner(&mut self) {
        let Some(ics) = self.message_invitation_ics() else {
            self.set_status("This message has no calendar invitation.");
            return;
        };
        let Some(client) = Planner123::discover(self.planner123_calendar.clone(), local_timezone())
        else {
            self.set_error(
                "Planner123 was not found. Install planner123-cli, or set SOLVERFORGE_PLANNER123_CLI.",
            );
            return;
        };

        self.loading = true;
        self.worker.add_to_planner(ics, client);
    }
}

/// The reader's IANA timezone name, for events that carry no timezone.
///
/// Planner123 needs a default zone for a floating time; asking it without one
/// would store the event in the wrong zone.
fn local_timezone() -> String {
    if let Ok(name) = std::env::var("TZ") {
        let name = name.trim().trim_start_matches(':');
        if !name.is_empty() {
            return name.to_string();
        }
    }
    std::fs::read_to_string("/etc/timezone")
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            // A symlink into the zoneinfo database names the zone.
            std::fs::read_link("/etc/localtime").ok().and_then(|path| {
                let text = path.to_string_lossy().to_string();
                text.split("zoneinfo/").nth(1).map(str::to_string)
            })
        })
        .unwrap_or_else(|| "UTC".to_string())
}
