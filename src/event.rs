use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent};

/// Application-level events fed into the main loop.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed (only key-down events).
    Key(KeyEvent),
    /// A mouse event (click, scroll, etc.).
    Mouse(MouseEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Periodic tick for animations / loading spinners.
    Tick,
}

/// Spawns a background thread that polls crossterm for terminal events
/// and sends them through an `mpsc` channel.  The tick interval drives
/// the loading-spinner animation.
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _tx: mpsc::Sender<Event>,
}

impl EventHandler {
    /// Create a new handler.  `tick_rate` controls how often `Event::Tick`
    /// is emitted when no terminal event arrives.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();

        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key)) => {
                        // Only forward key-down presses (ignore release / repeat).
                        if key.kind == KeyEventKind::Press
                            && event_tx.send(Event::Key(key)).is_err()
                        {
                            return;
                        }
                    }
                    Ok(CrosstermEvent::Mouse(mouse)) => {
                        if event_tx.send(Event::Mouse(mouse)).is_err() {
                            return;
                        }
                    }
                    Ok(CrosstermEvent::Resize(w, h))
                        if event_tx.send(Event::Resize(w, h)).is_err() =>
                    {
                        return;
                    }
                    _ => {}
                }
            } else {
                // No event within tick_rate — emit a tick.
                if event_tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Blocking receive of the next event.
    pub fn next(&self) -> anyhow::Result<Event> {
        self.rx.recv().map_err(Into::into)
    }
}
