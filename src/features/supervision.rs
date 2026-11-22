//! Restart policy for what the manager watches (ADR 0017, H6): wait longer after each restart
//! (exponential backoff) and, after too many restarts in an hour, stop and explain instead of
//! looping forever (circuit breaker). Pure: time is a parameter, so it is tested without waiting.

pub const MAX_RESTARTS: usize = 3;
pub const WINDOW_MS: u64 = 3_600_000;
pub const BASE_WAIT_MS: u64 = 2_000;
pub const MAX_WAIT_MS: u64 = 5 * 60_000;

/// Restarts of one thing in the last hour. It is saved between runs, so restarting the watcher
/// itself does not reset the count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Breaker {
    restarts: Vec<u64>,
}

impl Breaker {
    pub fn from_times(times: Vec<u64>) -> Self {
        Self { restarts: times }
    }

    pub fn times(&self) -> &[u64] {
        &self.restarts
    }

    fn prune(&mut self, now: u64) {
        self.restarts
            .retain(|at| now.saturating_sub(*at) < WINDOW_MS);
    }

    /// Whether one more restart is allowed.
    pub fn allows(&mut self, now: u64) -> bool {
        self.prune(now);
        self.restarts.len() < MAX_RESTARTS
    }

    pub fn record(&mut self, now: u64) {
        self.prune(now);
        self.restarts.push(now);
    }

    /// How long to wait before the next restart: 2 s, 4 s, 8 s… up to 5 min.
    pub fn wait_ms(&mut self, now: u64) -> u64 {
        self.prune(now);
        let doublings = u32::try_from(self.restarts.len())
            .unwrap_or(u32::MAX)
            .min(20);
        BASE_WAIT_MS
            .saturating_mul(1u64 << doublings)
            .min(MAX_WAIT_MS)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Healthy,
    /// Down; waiting before trying again.
    Waiting,
    Restarted,
    RestartFailed(String),
    /// Too many restarts: it stopped trying. The text explains it.
    Tripped(String),
}

/// Something the manager keeps alive.
pub trait Target {
    fn name(&self) -> &'static str;
    fn healthy(&self) -> bool;
    fn restart(&self) -> Result<(), String>;
}

/// One look at `target`. `down_since` is the caller's memory of when it was first seen down.
pub fn step(
    target: &dyn Target,
    breaker: &mut Breaker,
    down_since: &mut Option<u64>,
    now: u64,
) -> Outcome {
    if target.healthy() {
        *down_since = None;
        return Outcome::Healthy;
    }
    let since = *down_since.get_or_insert(now);
    if now.saturating_sub(since) < breaker.wait_ms(now) {
        return Outcome::Waiting;
    }
    if !breaker.allows(now) {
        return Outcome::Tripped(format!(
            "{} se cayó {MAX_RESTARTS} veces en la última hora y no lo vuelvo a levantar solo. \
             Mira su registro, corrige la causa y vuelve a ejecutar `chxchx-tech manager watch`.",
            target.name()
        ));
    }
    breaker.record(now);
    // Give it time to come up before judging it again.
    *down_since = Some(now);
    match target.restart() {
        Ok(()) => Outcome::Restarted,
        Err(error) => Outcome::RestartFailed(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct Fake {
        up: Cell<bool>,
        starts: RefCell<u32>,
        works: bool,
    }

    impl Target for Fake {
        fn name(&self) -> &'static str {
            "el nodo"
        }
        fn healthy(&self) -> bool {
            self.up.get()
        }
        fn restart(&self) -> Result<(), String> {
            *self.starts.borrow_mut() += 1;
            if self.works {
                Ok(())
            } else {
                Err("no arranca".into())
            }
        }
    }

    fn fake(works: bool) -> Fake {
        Fake {
            up: Cell::new(false),
            starts: RefCell::new(0),
            works,
        }
    }

    #[test]
    fn the_wait_doubles_and_is_capped() {
        let mut breaker = Breaker::default();
        assert_eq!(breaker.wait_ms(0), 2_000);
        breaker.record(0);
        assert_eq!(breaker.wait_ms(1), 4_000);
        breaker.record(1);
        assert_eq!(breaker.wait_ms(2), 8_000);
        let many = Breaker::from_times(vec![5; 40]);
        assert_eq!(many.clone().wait_ms(6), MAX_WAIT_MS);
    }

    #[test]
    fn a_healthy_target_is_left_alone() {
        let target = fake(true);
        target.up.set(true);
        let (mut breaker, mut down) = (Breaker::default(), None);
        assert_eq!(step(&target, &mut breaker, &mut down, 0), Outcome::Healthy);
        assert_eq!(*target.starts.borrow(), 0);
    }

    #[test]
    fn it_waits_then_restarts_then_stops_after_three_and_explains() {
        let target = fake(true);
        let (mut breaker, mut down) = (Breaker::default(), None);
        let mut now = 0;
        let mut outcomes = Vec::new();
        // The node never comes back; look every second for a minute.
        for _ in 0..60 {
            outcomes.push(step(&target, &mut breaker, &mut down, now));
            now += 1_000;
            if matches!(outcomes.last(), Some(Outcome::Tripped(_))) {
                break;
            }
        }
        assert_eq!(*target.starts.borrow(), 3, "exactly three restarts");
        assert!(outcomes.contains(&Outcome::Waiting));
        let Some(Outcome::Tripped(why)) = outcomes.last() else {
            panic!("it should have stopped: {outcomes:?}");
        };
        assert!(
            why.contains("el nodo") && why.contains("manager watch"),
            "{why}"
        );
    }

    #[test]
    fn the_hour_passing_gives_it_another_chance() {
        let mut breaker = Breaker::from_times(vec![0, 1, 2]);
        assert!(!breaker.allows(10));
        assert!(breaker.allows(WINDOW_MS + 10));
    }

    #[test]
    fn a_failed_restart_is_reported_and_still_counts() {
        let target = fake(false);
        let (mut breaker, mut down) = (Breaker::default(), None);
        let mut now = 0;
        let mut failed = false;
        for _ in 0..10 {
            if matches!(
                step(&target, &mut breaker, &mut down, now),
                Outcome::RestartFailed(_)
            ) {
                failed = true;
                break;
            }
            now += 1_000;
        }
        assert!(failed);
        assert_eq!(breaker.times().len(), 1);
    }
}
