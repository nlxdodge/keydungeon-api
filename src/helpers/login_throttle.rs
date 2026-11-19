use std::collections::{HashMap, VecDeque};
use std::env;
use std::net::IpAddr;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct LoginThrottle {
    max_failed_attempts: usize,
    window: Duration,
    failures: std::sync::Arc<Mutex<HashMap<(IpAddr, Option<String>), VecDeque<Instant>>>>,
}

impl LoginThrottle {
    pub fn from_env() -> Self {
        let max_failed_attempts = env::var("LOGIN_MAX_FAILED_ATTEMPTS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);
        let window_seconds = env::var("LOGIN_ATTEMPT_WINDOW_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(300);

        Self::new(max_failed_attempts, Duration::from_secs(window_seconds))
    }

    pub fn new(max_failed_attempts: usize, window: Duration) -> Self {
        Self {
            max_failed_attempts: max_failed_attempts.max(1),
            window: window.max(Duration::from_secs(1)),
            failures: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn retry_after(&self, ip: IpAddr, username: &str) -> Option<u64> {
        let now = Instant::now();
        let mut failures = self.lock();
        Self::prune(&mut failures, now, self.window);

        let keys = [(ip, None), (ip, Some(normalize_username(username)))];
        keys.into_iter()
            .filter_map(|key| failures.get(&key))
            .filter(|attempts| attempts.len() >= self.max_failed_attempts)
            .filter_map(|attempts| attempts.get(attempts.len() - self.max_failed_attempts))
            .map(|attempt| {
                let remaining = self.window.saturating_sub(now.duration_since(*attempt));
                (remaining.as_secs_f64().ceil() as u64).max(1)
            })
            .max()
    }

    pub fn record_failure(&self, ip: IpAddr, username: &str) {
        let now = Instant::now();
        let mut failures = self.lock();
        Self::prune(&mut failures, now, self.window);

        failures.entry((ip, None)).or_default().push_back(now);
        failures
            .entry((ip, Some(normalize_username(username))))
            .or_default()
            .push_back(now);
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<(IpAddr, Option<String>), VecDeque<Instant>>> {
        self.failures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn prune(
        failures: &mut HashMap<(IpAddr, Option<String>), VecDeque<Instant>>,
        now: Instant,
        window: Duration,
    ) {
        failures.retain(|_, attempts| {
            while attempts
                .front()
                .is_some_and(|attempt| now.duration_since(*attempt) >= window)
            {
                attempts.pop_front();
            }
            !attempts.is_empty()
        });
    }
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttles_ip_and_ip_username_buckets_for_the_window() {
        let ip = "192.0.2.10".parse().unwrap();
        let throttle = LoginThrottle::new(2, Duration::from_secs(60));

        throttle.record_failure(ip, "alice");
        assert_eq!(throttle.retry_after(ip, "alice"), None);
        throttle.record_failure(ip, "alice");
        assert!(throttle.retry_after(ip, "ALICE").is_some());
        assert!(throttle.retry_after(ip, "another-user").is_some());
    }

    #[test]
    fn does_not_lock_an_account_across_different_ips() {
        let first_ip = "192.0.2.10".parse().unwrap();
        let second_ip = "192.0.2.11".parse().unwrap();
        let throttle = LoginThrottle::new(1, Duration::from_secs(60));

        throttle.record_failure(first_ip, "alice");

        assert!(throttle.retry_after(first_ip, "alice").is_some());
        assert_eq!(throttle.retry_after(second_ip, "alice"), None);
    }
}
