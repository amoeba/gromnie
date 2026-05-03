use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Unique identifier for a timer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TimerId(u64);

impl From<TimerId> for u64 {
    fn from(value: TimerId) -> Self {
        value.0
    }
}

impl From<u64> for TimerId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
struct Timer {
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct TimerState {
    timers: HashMap<TimerId, Timer>,
    next_id: u64,
    fired_timers: HashSet<TimerId>,
    fired_events: Vec<(TimerId, String)>,
}

/// Manages timers for scripts
pub struct TimerManager {
    state: Arc<Mutex<TimerState>>,
}

impl TimerManager {
    /// Create a new timer manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TimerState::default())),
        }
    }

    /// Schedule a one-shot timer that fires after a delay
    pub fn schedule_timer(&self, delay: Duration, name: String) -> TimerId {
        let (id, cancel) = self.insert_timer();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            if cancel.load(Ordering::SeqCst) {
                return;
            }

            let mut state = state.lock().expect("timer state poisoned");
            state.fired_timers.insert(id);
            state.fired_events.push((id, name));
            state.timers.remove(&id);
        });
        id
    }

    /// Schedule a recurring timer that fires repeatedly at an interval
    pub fn schedule_recurring(&self, interval: Duration, name: String) -> TimerId {
        let (id, cancel) = self.insert_timer();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if cancel.load(Ordering::SeqCst) {
                    break;
                }

                let mut state = state.lock().expect("timer state poisoned");
                state.fired_timers.insert(id);
                state.fired_events.push((id, name.clone()));
                if !state.timers.contains_key(&id) {
                    break;
                }
            }
        });
        id
    }

    /// Cancel a timer
    pub fn cancel_timer(&self, id: TimerId) -> bool {
        let mut state = self.state.lock().expect("timer state poisoned");
        let Some(timer) = state.timers.remove(&id) else {
            return false;
        };

        timer.cancel.store(true, Ordering::SeqCst);
        state.fired_timers.remove(&id);
        true
    }

    /// Check if a timer has fired (and consume the fired state)
    pub fn check_timer(&self, id: TimerId) -> bool {
        self.state
            .lock()
            .expect("timer state poisoned")
            .fired_timers
            .remove(&id)
    }

    /// Drain fired timer notifications collected by async timer tasks.
    pub fn tick(&self, _now: Instant) -> Vec<(TimerId, String)> {
        let mut state = self.state.lock().expect("timer state poisoned");
        std::mem::take(&mut state.fired_events)
    }

    /// Get the number of active timers
    pub fn active_count(&self) -> usize {
        self.state
            .lock()
            .expect("timer state poisoned")
            .timers
            .len()
    }

    fn insert_timer(&self) -> (TimerId, Arc<AtomicBool>) {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = self.state.lock().expect("timer state poisoned");
        let id = TimerId(state.next_id);
        state.next_id += 1;
        state.timers.insert(
            id,
            Timer {
                cancel: Arc::clone(&cancel),
            },
        );
        (id, cancel)
    }

    fn cancel_all(&self) {
        let mut state = self.state.lock().expect("timer state poisoned");
        for timer in state.timers.values() {
            timer.cancel.store(true, Ordering::SeqCst);
        }
        state.timers.clear();
        state.fired_timers.clear();
        state.fired_events.clear();
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TimerManager {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn test_one_shot_timer() {
        let manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_millis(50), "test".to_string());

        // Should not fire immediately
        let fired = manager.tick(Instant::now());
        assert!(fired.is_empty());

        // 1) Yield so the spawned task runs and registers its sleep at T=50ms.
        // 2) Advance the clock to T=60ms, waking that sleep.
        // 3) Yield again so the now-ready timer task fires its event.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(60)).await;
        tokio::task::yield_now().await;
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);
        assert_eq!(fired[0].1, "test");

        // Should be removed after firing
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn test_recurring_timer() {
        let manager = TimerManager::new();
        let id = manager.schedule_recurring(Duration::from_millis(50), "recurring".to_string());

        // Should not fire immediately
        let fired = manager.tick(Instant::now());
        assert!(fired.is_empty());

        // 1) Yield so the spawned task runs and registers its sleep at T=50ms.
        // 2) Advance the clock to T=60ms, waking that sleep.
        // 3) Yield again so the now-ready timer task fires its event.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(60)).await;
        tokio::task::yield_now().await;
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);

        // Should still be active
        assert_eq!(manager.active_count(), 1);

        // Repeat: the task re-registered its sleep at T=60ms+50ms=T=110ms.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(60)).await;
        tokio::task::yield_now().await;
        let fired = manager.tick(Instant::now());
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].0, id);
    }

    #[tokio::test(start_paused = true)]
    async fn test_cancel_timer() {
        let manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_secs(10), "test".to_string());

        assert!(manager.cancel_timer(id));
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.cancel_timer(id)); // Already removed
    }

    #[tokio::test(start_paused = true)]
    async fn test_check_timer() {
        let manager = TimerManager::new();
        let id = manager.schedule_timer(Duration::from_millis(50), "test".to_string());

        // Not fired yet
        assert!(!manager.check_timer(id));

        // 1) Yield so the spawned task runs and registers its sleep at T=50ms.
        // 2) Advance the clock to T=60ms, waking that sleep.
        // 3) Yield again so the now-ready timer task fires its event.
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(60)).await;
        tokio::task::yield_now().await;
        manager.tick(Instant::now());

        // Should return true once
        assert!(manager.check_timer(id));
        // Should return false second time (consumed)
        assert!(!manager.check_timer(id));
    }
}
