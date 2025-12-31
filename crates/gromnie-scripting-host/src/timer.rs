use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Unique identifier for a timer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

/// Type of timer
#[derive(Debug, Clone)]
enum TimerType {
    /// One-shot timer that fires once and is removed
    OneShot { fire_at: Instant },
    /// Recurring timer that fires repeatedly at an interval
    Recurring {
        interval: Duration,
        next_fire: Instant,
    },
}

/// A timer with metadata
#[derive(Debug, Clone)]
struct Timer {
    #[allow(dead_code)]
    id: TimerId,
    name: String,
    timer_type: TimerType,
}

/// Manages timers for scripts
pub struct TimerManager {
    timers: HashMap<TimerId, Timer>,
    next_id: u64,
    fired_timers: Vec<TimerId>,
}

impl TimerManager {
    /// Create a new timer manager
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            next_id: 0,
            fired_timers: Vec::new(),
        }
    }

    /// Schedule a one-shot timer that fires after a delay
    pub fn schedule_timer(&mut self, delay: Duration, name: String) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;

        let timer = Timer {
            id,
            name,
            timer_type: TimerType::OneShot {
                fire_at: Instant::now() + delay,
            },
        };

        self.timers.insert(id, timer);
        id
    }

    /// Schedule a recurring timer that fires repeatedly at an interval
    pub fn schedule_recurring(&mut self, interval: Duration, name: String) -> TimerId {
        let id = TimerId(self.next_id);
        self.next_id += 1;

        let timer = Timer {
            id,
            name,
            timer_type: TimerType::Recurring {
                interval,
                next_fire: Instant::now() + interval,
            },
        };

        self.timers.insert(id, timer);
        id
    }

    /// Cancel a timer
    pub fn cancel_timer(&mut self, id: TimerId) -> bool {
        self.timers.remove(&id).is_some()
    }

    /// Check if a timer has fired (and consume the fired state)
    pub fn check_timer(&mut self, id: TimerId) -> bool {
        if let Some(pos) = self.fired_timers.iter().position(|&tid| tid == id) {
            self.fired_timers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Process timers and return list of fired timer IDs with their names
    /// This should be called periodically (e.g., on each event)
    pub fn tick(&mut self, now: Instant) -> Vec<(TimerId, String)> {
        let mut fired = Vec::new();
        let mut to_remove = Vec::new();

        for (id, timer) in self.timers.iter_mut() {
            match &mut timer.timer_type {
                TimerType::OneShot { fire_at } => {
                    if now >= *fire_at {
                        fired.push((*id, timer.name.clone()));
                        to_remove.push(*id);
                    }
                }
                TimerType::Recurring {
                    interval,
                    next_fire,
                } => {
                    if now >= *next_fire {
                        fired.push((*id, timer.name.clone()));
                        *next_fire = now + *interval;
                    }
                }
            }
        }

        // Remove one-shot timers that have fired
        for id in to_remove {
            self.timers.remove(&id);
        }

        // Store fired timer IDs for check_timer()
        self.fired_timers.extend(fired.iter().map(|(id, _)| *id));

        fired
    }

    /// Get the number of active timers
    pub fn active_count(&self) -> usize {
        self.timers.len()
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_one_shot_timer() {
        let mut manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_millis(50), "test".to_string());

        // Should not fire immediately
        let fired = manager.tick(Instant::now());
        assert!(fired.is_empty());

        // Wait and tick again
        sleep(Duration::from_millis(60));
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);
        assert_eq!(fired[0].1, "test");

        // Should be removed after firing
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_recurring_timer() {
        let mut manager = TimerManager::new();
        let id = manager.schedule_recurring(Duration::from_millis(50), "recurring".to_string());

        // Should not fire immediately
        let fired = manager.tick(Instant::now());
        assert!(fired.is_empty());

        // Wait and tick - should fire
        sleep(Duration::from_millis(60));
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);

        // Should still be active
        assert_eq!(manager.active_count(), 1);

        // Wait and fire again
        sleep(Duration::from_millis(60));
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);
    }

    #[test]
    fn test_cancel_timer() {
        let mut manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_secs(10), "test".to_string());

        assert!(manager.cancel_timer(id));
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.cancel_timer(id)); // Already removed
    }

    #[test]
    fn test_check_timer() {
        let mut manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_millis(50), "test".to_string());

        // Not fired yet
        assert!(!manager.check_timer(id));

        // Fire the timer
        sleep(Duration::from_millis(60));
        manager.tick(Instant::now());

        // Should return true once
        assert!(manager.check_timer(id));
        // Should return false second time (consumed)
        assert!(!manager.check_timer(id));
    }
}
