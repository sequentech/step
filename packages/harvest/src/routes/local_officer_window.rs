// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! A single-process, bounded rendezvous. The value is a move-only proof; a
//! second consumer needs a new authenticated observation from the nominee.
//! Never await with this lock held. Restart, clock anomaly, expiry or excess
//! occupancy loses availability, not authorization.
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

pub(super) struct Window<K, V> {
    slots: HashMap<K, (Instant, V)>,
    limit: usize,
    lifetime: Duration,
}
impl<K: Eq + Hash, V> Window<K, V> {
    pub(super) fn new(limit: usize, lifetime: Duration) -> Self {
        Self {
            slots: HashMap::new(),
            limit,
            lifetime,
        }
    }
    fn prune(&mut self, now: Instant) {
        let ttl = self.lifetime;
        self.slots.retain(|_, (at, _)| {
            now.checked_duration_since(*at).is_some_and(|age| age < ttl)
        });
    }
    pub(super) fn put(
        &mut self,
        k: K,
        v: V,
        observed: Instant,
        now: Instant,
    ) -> bool {
        self.prune(now);
        if !now
            .checked_duration_since(observed)
            .is_some_and(|age| age < self.lifetime)
            || (!self.slots.contains_key(&k) && self.slots.len() >= self.limit)
            || self
                .slots
                .get(&k)
                .is_some_and(|(other, _)| *other > observed)
        {
            return false;
        }
        self.slots.insert(k, (observed, v));
        true
    }
    pub(super) fn matches(
        &mut self,
        k: &K,
        now: Instant,
        allowed: impl FnOnce(&V) -> bool,
    ) -> bool {
        self.prune(now);
        self.slots.get(k).is_some_and(|(_, v)| allowed(v))
    }
    pub(super) fn take(
        &mut self,
        k: &K,
        now: Instant,
        allowed: impl FnOnce(&V) -> bool,
    ) -> Option<V> {
        self.prune(now);
        if !self.slots.get(k).is_some_and(|(_, v)| allowed(v)) {
            return None;
        }
        self.slots.remove(k).map(|(_, v)| v)
    }
    pub(super) fn visible<R>(
        &mut self,
        now: Instant,
        view: impl Fn(&V) -> Option<R>,
    ) -> Vec<R> {
        self.prune(now);
        self.slots.values().filter_map(|(_, v)| view(v)).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_use_only_and_wrong_officer_cannot_evict() {
        let t = Instant::now();
        let mut w = Window::new(2, Duration::from_secs(20));
        assert!(w.put(32, String::from("officer-a"), t, t));
        assert!(w.take(&32, t, |officer| officer == "officer-b").is_none());
        assert_eq!(w.visible(t, |_| Some(1)), [1]);
        assert_eq!(
            w.take(&32, t, |officer| officer == "officer-a").as_deref(),
            Some("officer-a")
        );
        assert!(w.take(&32, t, |_| true).is_none());
    }
    #[test]
    fn capacity_strict_expiry_and_stale_observations() {
        let t = Instant::now();
        let ttl = Duration::from_secs(20);
        let mut w = Window::new(1, ttl);
        assert!(w.put(2, 3, t, t));
        assert!(!w.put(3, 4, t, t));
        assert!(w.put(2, 4, t + ttl / 2, t + ttl / 2));
        assert!(!w.put(2, 8, t, t + ttl / 2));
        assert!(w.visible(t + ttl, |v| Some(*v)) == [4]);
        assert!(w.visible(t + ttl + ttl / 2, |v| Some(*v)).is_empty());
        assert!(!w.put(2, 4, t, t + ttl));
        assert!(w.put(3, 7, t + ttl, t + ttl));
    }
}
