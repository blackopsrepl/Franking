/*! Compatibility boundary for the retired local triage lanes. */

use crate::mail::types::Envelope;

use super::model::App;

impl App {
    /// Server folders define membership; legacy local routes must not hide mail.
    /// Keep this boundary while callers migrate away from triage lane state.
    pub(crate) fn filter_triage_lane(&mut self, envelopes: Vec<Envelope>) -> Option<Vec<Envelope>> {
        Some(envelopes)
    }
}
