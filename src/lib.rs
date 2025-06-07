//! # XTState
//!
//! `XTState` is a flexible, thread-safe state management utility for tracking named boolean slots and their activation history in Rust.
//!
//! ## Features
//! - Track multiple named boolean slots (flags) and their states.
//! - Record a timestamped history of all slot changes.
//! - Determine when all slots are active (true) via the `activated` field.
//! - Thread-safe usage via the `ThreadSafeXTState` type alias (`Arc<Mutex<XTState>>`).
//!
//! ## Example Usage
//! ```rust
//! use xtstate::{XTState, ThreadSafeXTState};
//! use std::collections::HashSet;
//! use std::sync::{Arc, Mutex};
//!
//! // Create a new XTState
//! let mut xt = XTState::new();
//! xt.setup_slots(HashSet::from(["slot1".to_string(), "slot2".to_string()]), false);
//! xt.update_callback("slot1".to_string(), true);
//! xt.update_callback("slot2".to_string(), true);
//! assert!(xt.activated);
//!
//! // Thread-safe usage
//! let state: ThreadSafeXTState = Arc::new(Mutex::new(XTState::new()));
//! {
//!     let mut xt = state.lock().unwrap();
//!     xt.setup_slots(HashSet::from(["slot1".to_string(), "slot2".to_string()]), false);
//! }
//! // ... spawn threads and update slots ...
//! ```
//!
//! ## Use Cases
//! - Feature flag management
//! - Workflow or step completion tracking
//! - Distributed system readiness coordination
//! - Event synchronization
//!
//! ## Crate Features
//! - Requires the `chrono` crate for timestamping history entries.
//!
//! ## Thread Safety
//! Use the `ThreadSafeXTState` type alias for safe sharing and mutation across threads.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub type ThreadSafeXTState = Arc<Mutex<XTState>>;

type Identifier = String;

pub struct XTState {
    slots: HashMap<Identifier, bool>,
    history: Vec<(Identifier, bool, i64)>,
    is_setup: bool,
    activated: bool,
}

impl XTState {
    pub fn new() -> Self {
        XTState {
            slots: HashMap::new(),
            history: Vec::new(),
            is_setup: false,
            activated: false,
        }
    }

    pub fn setup_slots(&mut self, slots: HashSet<Identifier>, force: bool) {
        if !force && self.is_setup {
            panic!("xtstate is already set up. use force to override.");
        }
        if force && self.is_setup {
            self.is_setup = false;
            self.activated = false;
            self.history.clear();
            self.slots.clear();
        }
        for slot in slots {
            self.slots.insert(slot, false);
        }
        self.is_setup = true;
    }

    fn can_activate(&self) -> bool {
        if !self.is_setup {
            panic!("xtstate is not set up. call setup_slots first.");
        }
        if self.slots.is_empty() {
            panic!("no slots are defined. call setup_slots with valid slots.");
        }

        self.slots.values().all(|&v| v)
    }

    pub fn update_callback(&mut self, identifier: Identifier, value: bool) {
        if !self.is_setup {
            panic!("xtstate is not set up. call setup_slots first.");
        }
        if !self.slots.contains_key(&identifier) {
            panic!("identifier '{}' is not defined in the slots.", identifier);
        }

        let epoch = chrono::Utc::now().timestamp_millis();

        self.history.push((identifier.clone(), value, epoch));

        if let Some(slot_value) = self.slots.get_mut(&identifier) {
            *slot_value = value;
        } else {
            panic!("identifier '{}' not found in slots.", identifier);
        }

        if self.can_activate() {
            self.activated = true;
        } else {
            self.activated = false;
        }
    }
}

impl Default for XTState {
    fn default() -> Self {
        XTState::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_basic() {
        let mut xt_state = XTState::new();
        xt_state.setup_slots(HashSet::from(["slot1".to_string(), "slot2".to_string()]), false);
        
        xt_state.update_callback("slot1".to_string(), true);
        xt_state.update_callback("slot2".to_string(), true);
        
        assert!(xt_state.activated);
    }

    #[test]
    fn test_threadsafe_xtstate() {
        use std::thread;
        use std::time::Duration;

        let state: ThreadSafeXTState = Arc::new(Mutex::new(XTState::new()));
        {
            let mut xt = state.lock().unwrap();
            xt.setup_slots(HashSet::from(["slot1".to_string(), "slot2".to_string()]), false);
        }

        let state1 = Arc::clone(&state);
        let handle1 = thread::spawn(move || {
            let mut xt = state1.lock().unwrap();
            xt.update_callback("slot1".to_string(), true);
        });

        let state2 = Arc::clone(&state);
        let handle2 = thread::spawn(move || {
            // Simulate some work
            thread::sleep(Duration::from_millis(10));
            let mut xt = state2.lock().unwrap();
            xt.update_callback("slot2".to_string(), true);
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        let xt = state.lock().unwrap();
        assert!(xt.activated);
    }
}