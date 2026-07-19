//! Cross-platform Instant abstraction that works in both native and WASM.
//!
//! On native: wraps std::time::Instant
//! On WASM: wraps u64 milliseconds from js_sys::Date::now()

use std::future::Future;
use std::ops::{Add, Sub};
use std::pin::Pin;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    inner: u64, // milliseconds since epoch
}

impl Instant {
    pub fn now() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            inner: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            inner: js_sys::Date::now() as u64,
        }
    }

    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let now = js_sys::Date::now() as u64;
            let elapsed_ms = now.saturating_sub(self.inner);
            Duration::from_millis(elapsed_ms)
        }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.duration_since(earlier.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let elapsed_ms = self.inner.saturating_sub(earlier.inner);
            Duration::from_millis(elapsed_ms)
        }
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Instant {
                inner: self.inner - rhs,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Instant {
                inner: self.inner.saturating_sub(rhs.as_millis() as u64),
            }
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Instant {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Instant {
                inner: self.inner + rhs,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Instant {
                inner: self.inner + rhs.as_millis() as u64,
            }
        }
    }
}

/// Cross-platform sleep that works in both native and WASM.
///
/// On native: uses tokio::time::sleep
/// On WASM: no-op (timers not available in WASM Send context; use spawn_local with gloo_timers directly)
pub fn sleep(_duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::pin(async move {
            tokio::time::sleep(_duration).await;
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On WASM, we can't use gloo_timers here because TimeoutFuture is not Send
        // and this future needs to be Send for tokio::spawn compatibility.
        // WASM callers should use spawn_local with gloo_timers directly instead.
        Box::pin(async {})
    }
}

/// Spawn a detached async task. On native uses tokio::spawn, on WASM uses spawn_local.
pub fn spawn_detached<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::spawn(future);
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}
