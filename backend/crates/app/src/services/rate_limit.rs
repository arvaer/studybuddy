//! Attempt limiting for login and signup (#37).
//!
//! Two sliding windows: attempts per source address and attempts per email
//! (case-folded). An attempt counts when it is made, whether or not it
//! succeeds, so a flood of wrong passwords and a flood of signups look the
//! same to the limiter. A successful login clears that email's window, so a
//! learner who finally gets the password right is not locked out by their own
//! earlier typos. State is in-process: a restart forgets it, and several
//! backend processes do not share it. That is documented, not hidden.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthLimits {
    /// Attempts allowed per source address per window.
    pub per_ip:    u32,
    /// Attempts allowed per email per window.
    pub per_email: u32,
    pub window:    Duration,
}

impl Default for AuthLimits {
    fn default() -> Self {
        Self { per_ip: 50, per_email: 5, window: Duration::from_secs(15 * 60) }
    }
}

#[derive(Debug)]
pub struct AuthLimiter {
    limits:  AuthLimits,
    windows: Mutex<HashMap<String, Vec<Instant>>>,
}

impl AuthLimiter {
    pub fn new(limits: AuthLimits) -> Self {
        Self { limits, windows: Mutex::new(HashMap::new()) }
    }

    pub fn limits(&self) -> &AuthLimits {
        &self.limits
    }

    /// Record one attempt from `ip` for `email` at `now`. `Err` carries how
    /// long until the exhausted window frees a slot; nothing is recorded then.
    pub fn check(&self, ip: &str, email: &str, now: Instant) -> Result<(), Duration> {
        let ip_key = format!("ip:{ip}");
        let email_key = format!("email:{}", email.trim().to_ascii_lowercase());
        let mut windows = self.windows.lock().unwrap_or_else(|e| e.into_inner());

        let window = self.limits.window;
        let prune = |v: &mut Vec<Instant>| v.retain(|t| now.duration_since(*t) < window);

        let ip_hits = windows.entry(ip_key.clone()).or_default();
        prune(ip_hits);
        if ip_hits.len() >= self.limits.per_ip as usize {
            return Err(window - now.duration_since(ip_hits[0]));
        }
        let email_hits = windows.entry(email_key.clone()).or_default();
        prune(email_hits);
        if email_hits.len() >= self.limits.per_email as usize {
            return Err(window - now.duration_since(email_hits[0]));
        }

        windows.get_mut(&ip_key).unwrap().push(now);
        windows.get_mut(&email_key).unwrap().push(now);
        Ok(())
    }

    /// A successful login: the email's window is cleared. The address window
    /// is kept, since one source may be trying many accounts.
    pub fn succeeded(&self, email: &str) {
        let key = format!("email:{}", email.trim().to_ascii_lowercase());
        self.windows.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter() -> AuthLimiter {
        AuthLimiter::new(AuthLimits { per_ip: 4, per_email: 2, window: Duration::from_secs(60) })
    }

    #[test]
    fn per_email_limit_trips_and_frees_when_the_window_passes() {
        let l = limiter();
        let t0 = Instant::now();
        assert!(l.check("1.1.1.1", "A@x.test", t0).is_ok());
        assert!(l.check("2.2.2.2", "a@x.test", t0 + Duration::from_secs(10)).is_ok());
        let wait = l.check("3.3.3.3", "a@x.test", t0 + Duration::from_secs(20)).unwrap_err();
        assert_eq!(wait, Duration::from_secs(40));
        // The refused attempt was not recorded: at t0+60 the first hit ages out.
        assert!(l.check("3.3.3.3", "a@x.test", t0 + Duration::from_secs(60)).is_ok());
    }

    #[test]
    fn per_ip_limit_trips_across_emails() {
        let l = limiter();
        let t0 = Instant::now();
        for i in 0..4 {
            assert!(l.check("9.9.9.9", &format!("u{i}@x.test"), t0).is_ok());
        }
        assert!(l.check("9.9.9.9", "u5@x.test", t0 + Duration::from_secs(1)).is_err());
        assert!(l.check("8.8.8.8", "u5@x.test", t0 + Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn success_clears_the_email_window_but_not_the_address_window() {
        let l = limiter();
        let t0 = Instant::now();
        assert!(l.check("1.1.1.1", "a@x.test", t0).is_ok());
        assert!(l.check("1.1.1.1", "a@x.test", t0).is_ok());
        assert!(l.check("1.1.1.1", "a@x.test", t0).is_err());
        l.succeeded("a@x.test");
        assert!(l.check("1.1.1.1", "a@x.test", t0).is_ok());
        assert!(l.check("1.1.1.1", "b@x.test", t0).is_ok());
        assert!(l.check("1.1.1.1", "c@x.test", t0).is_err(), "address window still counts");
    }
}
