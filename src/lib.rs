use carapax::prelude::*;
use ratelimit_meter::{DirectRateLimiter, KeyedRateLimiter, GCRA};
use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

pub use nonzero_ext::nonzero;

/// Limits number of updates per time
pub struct RateLimitHandler {
    rate_limiter: RateLimiter,
}

impl RateLimitHandler {
    /// Limit all updates
    ///
    /// # Arguments
    ///
    /// - capacity - Number of updates
    /// - seconds - Duration in seconds
    pub fn direct(capacity: NonZeroU32, seconds: u64) -> Self {
        Self {
            rate_limiter: RateLimiter::Direct(Arc::new(Mutex::new(DirectRateLimiter::new(
                capacity,
                Duration::from_secs(seconds),
            )))),
        }
    }

    /// Limit updates for each user or chat
    ///
    /// # Arguments
    ///
    /// - key - User or Chat
    /// - capacity - Number of updates
    /// - seconds - Duration in seconds
    /// - on_missing - Allow or deny update when user or chat not found
    ///                (got an update from channel or inline query, etc...)
    pub fn keyed(key: RateLimitKey, capacity: NonZeroU32, seconds: u64, on_missing: bool) -> Self {
        Self {
            rate_limiter: RateLimiter::Keyed {
                limiter: Arc::new(Mutex::new(KeyedRateLimiter::new(
                    capacity,
                    Duration::from_secs(seconds),
                ))),
                on_missing,
                key,
            },
        }
    }
}

impl UpdateHandler for RateLimitHandler {
    fn handle(&self, _context: &mut Context, update: &Update) -> HandlerFuture {
        let should_pass = match self.rate_limiter {
            RateLimiter::Direct(ref limiter) => limiter.lock().unwrap().check().is_ok(),
            RateLimiter::Keyed {
                ref limiter,
                key,
                on_missing,
            } => {
                let mut limiter = limiter.lock().unwrap();
                let val = match key {
                    RateLimitKey::Chat => update.get_chat_id(),
                    RateLimitKey::User => update.get_user().map(|u| u.id),
                };
                if let Some(val) = val {
                    limiter.check(val).is_ok()
                } else {
                    on_missing
                }
            }
        };
        if should_pass {
            HandlerResult::Continue
        } else {
            HandlerResult::Stop
        }
        .into()
    }
}

enum RateLimiter {
    Direct(Arc<Mutex<DirectRateLimiter<GCRA>>>),
    Keyed {
        limiter: Arc<Mutex<KeyedRateLimiter<Integer, GCRA>>>,
        key: RateLimitKey,
        on_missing: bool,
    },
}

/// Rate limit key
#[derive(Copy, Clone, Debug)]
pub enum RateLimitKey {
    /// Limit per chat
    Chat,
    /// Limit per user
    User,
}
