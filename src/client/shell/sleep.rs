use super::*;
use std::hash::BuildHasher;
use std::time::{Duration, Instant};

pub(super) const SLEEP_TIMERS: &[u64] = &[5, 10, 15, 30];

pub(super) const SLEEP_PHRASES: &[&str] = &[
    "You're done here. They're not.",
    "The server is busy. You don't have to be.",
    "Your attention is no longer a dependency.",
    "No action required. A rare moment.",
    "Reality has no staging environment. 🪐",
    "Time keeps shipping without release notes.",
    "The agents have it from here.",
    "Watching them won't make them finish faster.",
    "Even stars eventually log off.",
    "Somewhere, a star just went offline.",
    "Your continued attendance is not required.",
    "Things are happening in roughly the intended order.",
    "The machine can hold the thought for a while.",
    "The process can continue without you.",
    "What are you waiting for?",
    "When is enough enough?",
    "Nothing needs you right now.",
    "The work can continue without your attention.",
    "The agents don't need an audience.",
    "You can stop watching now.",
    "You've done your part.",
    "Still running. You're free. ✨",
    "Agents at work. Human optional.",
    "You may safely look away.",
    "Nothing here needs supervising.",
    "You're not the bottleneck anymore.",
    "You can leave this one to them.",
    "Somewhere, something is still running.",
    "This part doesn't need you.",
    "Go take a look at the trees.",
];

impl ClientShellState {
    pub(super) fn begin_sleep(&mut self, now: Instant) {
        if self.config.top_padding == 0 || self.snapshot.is_none() {
            return;
        }
        self.sleep_started = Some(now);
        // Randomized once per opening, never during rendering or countdown ticks.
        let choice = std::collections::hash_map::RandomState::new().hash_one(now);
        self.sleep_phrase = SLEEP_PHRASES[choice as usize % SLEEP_PHRASES.len()];
        self.sleep_frame = 0;
        self.sleep_remaining = self.sleep_seconds;
        self.chrome_drag = None;
        self.agent_press = None;
        self.tab_press = None;
        self.stop_selection_autoscroll();
    }

    pub(crate) fn tick_sleep(&mut self, now: Instant) -> ClientShellInput {
        let Some(started) = self.sleep_started else {
            return ClientShellInput::default();
        };
        let elapsed = now.saturating_duration_since(started);
        self.sleep_remaining = self.sleep_seconds.saturating_sub(elapsed.as_secs());
        self.sleep_frame = (elapsed.as_millis() / 200 % 4) as usize;
        // Detach only the client. Never issue a pane, workspace, or server close.
        ClientShellInput {
            detach: elapsed >= Duration::from_secs(self.sleep_seconds),
            repaint: true,
            ..ClientShellInput::default()
        }
    }
}
