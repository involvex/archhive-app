//! Per-host request pacing for site scraping.
//!
//! Trending rows, feeds, saved-search checks and category lookups can all hit
//! the same site within a second, which tube sites answer with HTTP 429. This
//! spaces requests to one host at least [`MIN_GAP`] apart and, after a 429,
//! fails fast during a cooldown instead of hammering the site further.

use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Minimum spacing between request starts to the same host.
const MIN_GAP: Duration = Duration::from_millis(1000);
/// Cooldown after a 429 when the site sends no usable `Retry-After`.
const DEFAULT_COOLDOWN: Duration = Duration::from_secs(60);
/// Upper bound on a `Retry-After` we are willing to honour.
const MAX_COOLDOWN: Duration = Duration::from_secs(15 * 60);

#[derive(Default, Clone, Copy)]
struct HostState {
    next_slot: Option<Instant>,
    cooldown_until: Option<Instant>,
}

static HOSTS: LazyLock<Mutex<HashMap<String, HostState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// `https://www.pornhub.com/x` → `pornhub.com` (so www/no-www share a budget).
fn host_key(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    host.strip_prefix("www.").unwrap_or(&host).to_string()
}

/// Reserve the next send slot for a host. Returns how long to wait, or the
/// remaining cooldown as an error.
fn reserve(state: &mut HostState, now: Instant) -> Result<Duration, Duration> {
    if let Some(until) = state.cooldown_until {
        if until > now {
            return Err(until - now);
        }
        state.cooldown_until = None;
    }
    let slot = state.next_slot.filter(|s| *s > now).unwrap_or(now);
    state.next_slot = Some(slot + MIN_GAP);
    Ok(slot - now)
}

/// Wait for this host's next slot (or fail fast while it is cooling down).
pub async fn acquire(url: &str) -> AppResult<()> {
    let key = host_key(url);
    let wait = {
        let mut hosts = HOSTS.lock().map_err(|e| AppError::Other(e.to_string()))?;
        reserve(hosts.entry(key.clone()).or_default(), Instant::now())
    };
    match wait {
        Ok(d) if d.is_zero() => Ok(()),
        Ok(d) => {
            tokio::time::sleep(d).await;
            Ok(())
        }
        Err(remaining) => Err(AppError::Site(format!(
            "{key} is rate limiting requests; try again in {}s",
            remaining.as_secs().max(1)
        ))),
    }
}

/// Record a 429 so later requests to the host back off.
pub fn note_rate_limited(url: &str, retry_after: Option<&str>) {
    let cooldown = retry_after
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_COOLDOWN)
        .clamp(MIN_GAP, MAX_COOLDOWN);
    if let Ok(mut hosts) = HOSTS.lock() {
        let state = hosts.entry(host_key(url)).or_default();
        state.cooldown_until = Some(Instant::now() + cooldown);
    }
    tracing::warn!(
        "[throttle] {} returned 429; backing off {}s",
        host_key(url),
        cooldown.as_secs()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_normalises() {
        assert_eq!(host_key("https://www.pornhub.com/video?c=3"), "pornhub.com");
        assert_eq!(host_key("https://XNXX.com:443/search/x"), "xnxx.com");
        assert_eq!(
            host_key("https://api.redgifs.com/v2/gifs"),
            "api.redgifs.com"
        );
    }

    #[test]
    fn spaces_requests_and_honours_cooldown() {
        let now = Instant::now();
        let mut s = HostState::default();
        assert_eq!(reserve(&mut s, now), Ok(Duration::ZERO));
        assert_eq!(reserve(&mut s, now), Ok(MIN_GAP));
        assert_eq!(reserve(&mut s, now), Ok(MIN_GAP * 2));
        // After the queue drains, a later request goes straight out.
        assert_eq!(reserve(&mut s, now + MIN_GAP * 5), Ok(Duration::ZERO));

        s.cooldown_until = Some(now + Duration::from_secs(30));
        assert_eq!(reserve(&mut s, now), Err(Duration::from_secs(30)));
        assert!(reserve(&mut s, now + Duration::from_secs(31)).is_ok());
    }
}
