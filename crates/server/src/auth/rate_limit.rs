use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use dashmap::{DashMap, mapref::entry::Entry};

struct RateWindow {
    started_at: Instant,
    count: usize,
}

pub struct LoginRateLimiter {
    entries: DashMap<IpAddr, RateWindow>,
    limit: usize,
    window: Duration,
}

impl LoginRateLimiter {
    pub fn per_minute(limit: usize) -> Self {
        Self {
            entries: DashMap::new(),
            limit,
            window: Duration::from_secs(60),
        }
    }

    pub fn allow(&self, address: IpAddr, now: Instant) -> bool {
        match self.entries.entry(address) {
            Entry::Vacant(entry) => {
                entry.insert(RateWindow {
                    started_at: now,
                    count: 1,
                });
                true
            }
            Entry::Occupied(mut entry) => update_window(entry.get_mut(), now, self),
        }
    }

    pub fn cleanup(&self, now: Instant) -> usize {
        let addresses = self
            .entries
            .iter()
            .filter(|entry| now.duration_since(entry.started_at) >= self.window)
            .map(|entry| *entry.key())
            .collect::<Vec<_>>();
        for address in &addresses {
            self.entries.remove(address);
        }
        addresses.len()
    }
}

fn update_window(window: &mut RateWindow, now: Instant, limiter: &LoginRateLimiter) -> bool {
    if now.duration_since(window.started_at) >= limiter.window {
        window.started_at = now;
        window.count = 1;
        return true;
    }
    if window.count >= limiter.limit {
        return false;
    }
    window.count += 1;
    true
}
