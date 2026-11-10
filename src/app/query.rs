//! Per-resource query cache with TanStack-Query-like semantics (stale time, polling,
//! no overlap, late results from an old scope dropped by generation).
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use crate::models::{DeploymentDetails, PodDetails};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// The result belongs to a canceled fetch or an old key; it was dropped.
    Stale,
    Ok,
    Err,
}

/// "No data yet" for the error-dialog rule: an empty list counts as no cache.
pub trait CacheEmpty {
    fn cache_empty(&self) -> bool;
}
impl<T> CacheEmpty for Vec<T> {
    fn cache_empty(&self) -> bool {
        self.is_empty()
    }
}
impl CacheEmpty for PodDetails {
    fn cache_empty(&self) -> bool {
        false
    }
}
impl CacheEmpty for DeploymentDetails {
    fn cache_empty(&self) -> bool {
        false
    }
}

// ponytail: the cache has no GC; per-key entries are tiny. Add a GC if the RAM gate in Task 16 says otherwise.
pub struct Query<K, T> {
    stale: Duration,
    poll: bool,
    entries: HashMap<K, (T, Instant)>,
    active: Option<K>,
    /// Activated (or cleared) and not started since: rule (a) applies.
    fresh: bool,
    /// Last `start` or activation: the poll timer origin.
    anchor: Option<Instant>,
    in_flight: Option<(K, u64)>,
    next_gen: u64,
    error: Option<(K, String)>,
}

impl<K: Clone + Eq + Hash, T: Clone> Query<K, T> {
    pub fn new(stale: Duration, poll: bool) -> Self {
        Self {
            stale,
            poll,
            entries: HashMap::new(),
            active: None,
            fresh: false,
            anchor: None,
            in_flight: None,
            next_gen: 0,
            error: None,
        }
    }

    /// A key change is a fresh activation: re-anchors the poll timer and drops the in-flight fetch.
    pub fn set_active(&mut self, key: Option<K>, now: Instant) {
        if key == self.active {
            return;
        }
        self.active = key;
        self.fresh = true;
        self.anchor = Some(now);
        self.in_flight = None;
    }

    pub fn data(&self) -> Option<&T> {
        self.entries.get(self.active.as_ref()?).map(|(v, _)| v)
    }

    pub fn is_loading(&self) -> bool {
        self.is_fetching() && self.data().is_none()
    }

    pub fn is_fetching(&self) -> bool {
        self.in_flight.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        match (&self.error, &self.active) {
            (Some((k, e)), Some(a)) if k == a => Some(e),
            _ => None,
        }
    }

    pub fn needs_fetch(&self, now: Instant, interval: Duration) -> bool {
        let Some(key) = &self.active else { return false };
        if self.in_flight.is_some() {
            return false;
        }
        let missing_or_stale = self
            .entries
            .get(key)
            .is_none_or(|(_, at)| now.saturating_duration_since(*at) >= self.stale);
        let poll_due = self.poll && self.anchor.is_some_and(|a| now.saturating_duration_since(a) >= interval);
        (self.fresh && missing_or_stale) || poll_due
    }

    pub fn start(&mut self, now: Instant) -> Option<(K, u64)> {
        let key = self.active.clone()?;
        let generation = self.next_gen;
        self.next_gen += 1;
        self.in_flight = Some((key.clone(), generation));
        self.fresh = false;
        self.anchor = Some(now);
        Some((key, generation))
    }

    pub fn finish(&mut self, key: &K, generation: u64, result: Result<T, String>, now: Instant) -> Outcome {
        match &self.in_flight {
            Some((k, g)) if k == key && *g == generation => self.in_flight = None,
            _ => return Outcome::Stale,
        }
        match result {
            Ok(v) => {
                self.entries.insert(key.clone(), (v, now));
                self.error = None;
                Outcome::Ok
            }
            Err(e) => {
                self.error = Some((key.clone(), e));
                Outcome::Err
            }
        }
    }

    pub fn cancel(&mut self) {
        self.in_flight = None;
    }

    /// Retry: forget everything so the active key fetches again on the next tick.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.error = None;
        self.in_flight = None;
        self.fresh = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);
    const POLL: Duration = Duration::from_secs(5);

    fn q(stale: Duration, poll: bool) -> (Query<String, Vec<u8>>, Instant) {
        (Query::new(stale, poll), Instant::now())
    }
    fn k(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn fetches_on_first_activation() {
        let (mut q, t) = q(Duration::MAX, false);
        assert!(!q.needs_fetch(t, POLL));
        q.set_active(k("a"), t);
        assert!(q.needs_fetch(t, POLL));
        assert_eq!(q.start(t), Some(("a".into(), 0)));
        assert!(q.is_loading() && q.is_fetching());
    }

    #[test]
    fn fresh_cache_skips_refetch_within_stale_time() {
        let (mut q, t) = q(30 * S, false);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        assert_eq!(q.finish(&key, g, Ok(vec![1]), t), Outcome::Ok);
        q.set_active(k("b"), t + 5 * S);
        q.set_active(k("a"), t + 10 * S);
        assert!(!q.needs_fetch(t + 10 * S, POLL));
        assert_eq!(q.data(), Some(&vec![1]));
    }

    #[test]
    fn zero_stale_refetches_on_switch_back_but_keeps_showing_cache() {
        let (mut q, t) = q(Duration::ZERO, false);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.finish(&key, g, Ok(vec![1]), t);
        q.set_active(k("b"), t + S);
        q.set_active(k("a"), t + 2 * S);
        assert!(q.needs_fetch(t + 2 * S, POLL));
        q.start(t + 2 * S);
        assert!(q.is_fetching() && !q.is_loading());
        assert_eq!(q.data(), Some(&vec![1]));
    }

    #[test]
    fn polls_after_interval_from_last_start() {
        let (mut q, t) = q(Duration::ZERO, true);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.finish(&key, g, Ok(vec![1]), t + S);
        assert!(!q.needs_fetch(t + Duration::from_millis(4900), POLL));
        assert!(q.needs_fetch(t + POLL, POLL));
    }

    #[test]
    fn no_overlap_while_in_flight() {
        let (mut q, t) = q(Duration::ZERO, true);
        q.set_active(k("a"), t);
        q.start(t);
        assert!(!q.needs_fetch(t + 12 * S, POLL));
    }

    #[test]
    fn stale_generation_is_ignored() {
        let (mut q, t) = q(Duration::ZERO, true);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.cancel();
        assert_eq!(q.finish(&key, g, Ok(vec![9]), t), Outcome::Stale);
        assert_eq!(q.data(), None);

        let (key, g) = q.start(t).unwrap();
        q.set_active(k("b"), t);
        assert_eq!(q.finish(&key, g, Err("late".into()), t), Outcome::Stale);
        q.set_active(k("a"), t);
        assert_eq!(q.data(), None);
        assert_eq!(q.error(), None);
    }

    #[test]
    fn error_keeps_previous_data() {
        let (mut q, t) = q(Duration::ZERO, true);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.finish(&key, g, Ok(vec![1]), t);
        let (key, g) = q.start(t + POLL).unwrap();
        assert_eq!(q.finish(&key, g, Err("boom".into()), t + POLL), Outcome::Err);
        assert_eq!(q.data(), Some(&vec![1]));
        assert_eq!(q.error(), Some("boom"));
    }

    #[test]
    fn no_refetch_after_error_without_poll() {
        let (mut q, t) = q(Duration::ZERO, false);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.finish(&key, g, Err("boom".into()), t);
        assert!(!q.needs_fetch(t + 60 * S, POLL));
    }

    #[test]
    fn clear_drops_cache() {
        let (mut q, t) = q(Duration::MAX, false);
        q.set_active(k("a"), t);
        let (key, g) = q.start(t).unwrap();
        q.finish(&key, g, Ok(vec![1]), t);
        q.clear();
        assert_eq!(q.data(), None);
        assert_eq!(q.error(), None);
        assert!(q.needs_fetch(t, POLL));
    }
}
