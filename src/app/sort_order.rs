/*! Message-list ordering commands. */

use crate::mail::sort::SortOrder;

use super::model::App;

impl App {
    /// Cycle the ordering and re-sort the listed page.
    pub(crate) fn cycle_sort_order(&mut self) {
        if self.threaded {
            self.set_status("Ordering follows the server in threaded view (press t).");
            return;
        }
        self.sort_order = self.sort_order.cycle();
        let order = self.sort_order;
        self.sort_envelopes(order);
        self.set_status(&format!("Order: {}.", order.label()));
    }

    /// Apply an ordering to the envelopes in place.
    pub(crate) fn sort_envelopes(&mut self, order: SortOrder) {
        order.apply(&mut self.envelopes);
        if self.envelopes.is_empty() {
            self.envelope_state.select(None);
        } else {
            self.envelope_state.select(Some(0));
        }
    }
}
