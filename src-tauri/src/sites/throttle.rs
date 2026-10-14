//! Per-host request pacing for site scraping.
//!
//! Trending rows, feeds, saved-search checks and category lookups can all hit
//! the same site within a second, which tube sites answer with HTTP 429. This
//! spaces requests to one host at least [`MIN_GAP`] apart and, after a 429,
//! fails fast during a cooldown instead of hammering the site further. One
//! probe request every [`PROBE_EVERY`] still goes through, and a successful
//! one ends the cooldown, so the app recovers as soon as the site does.

use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Minimum spacing between request starts to the same host.
const MIN_GAP: Duration = Duration::from_millis(1200);
/// Extra random wait on top of [`MIN_GAP`], so requests don't arrive on a
/// fixed beat like a script.
const JITTER_MS: u64 = 1600;
/// Cooldown after a 429 when the site sends no usable `Retry-After`.
const DEFAULT_COOLDOWN: Duration = Duration::from_secs(60);
/// Upper bound on a `Retry-After` we are willing to honour.
const MAX_COOLDOWN: Duration = Duration::from_secs(5 * 60);
/// During a cooldown, let one request through this often to test the site.
const PROBE_EVERY: Duration = Duration::from_secs(15);

#[derive(Default, Clone, Copy)]
struct HostState {
    next_slot: Option<Instant>,
    cooldown_until: Option<Instant>,
    last_probe: Option<Instant>,
}

/// Last page fetched per host (sent as Referer on the next one).
static LAST_PAGE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The previous page fetched from this URL's host, if any.
pub fn last_page(url: &str) -> Option<String> {
    LAST_PAGE.lock().ok()?.get(&host_key(url)).cloned()
}

/// Remember a successfully fetched page as the host's current page.
pub fn note_page(url: &str) {
    if let Ok(mut pages) = LAST_PAGE.lock() {
        pages.insert(host_key(url), url.to_string());
    }
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

/// Spacing for hosts with stricter limits than [`MIN_GAP`] + jitter:
/// (host, minimum gap, extra random jitter). xPomelo answered 429 after
/// about a dozen requests in 25 s at the default pace, and Cloudflare 1015
/// after ~15 requests at 4-7 s apart.
/// Pornwitch showed no limit, but is a small site: kept unhurried too.
const SLOW_HOSTS: &[(&str, Duration, u64)] = &[
    ("xpomelo.com", Duration::from_millis(6000), 4000),
    ("pornwitch.com", Duration::from_millis(2500), 2000),
];

/// Test-request interval while backing off: strict hosts count every
/// request against the block, so they are asked less often.
fn probe_every(host: &str) -> Duration {
    if SLOW_HOSTS.iter().any(|(h, _, _)| *h == host) {
        Duration::from_secs(90)
    } else {
        PROBE_EVERY
    }
}

fn pace_for(host: &str) -> (Duration, u64) {
    SLOW_HOSTS
        .iter()
        .find(|(h, _, _)| *h == host)
        .map(|(_, gap, jitter)| (*gap, *jitter))
        .unwrap_or((MIN_GAP, JITTER_MS))
}

/// [`reserve_paced`] at the default pace.
#[cfg(test)]
fn reserve(state: &mut HostState, now: Instant) -> Result<Duration, Duration> {
    reserve_paced(state, now, MIN_GAP, JITTER_MS, PROBE_EVERY)
}

/// Reserve the next send slot for a host. Returns how long to wait, or the
/// remaining cooldown as an error.
fn reserve_paced(
    state: &mut HostState,
    now: Instant,
    gap: Duration,
    jitter_ms: u64,
    probe: Duration,
) -> Result<Duration, Duration> {
    if let Some(until) = state.cooldown_until {
        if until > now {
            let probe_due = state
                .last_probe
                .is_none_or(|p| now.duration_since(p) >= probe);
            if !probe_due {
                return Err(until - now);
            }
            state.last_probe = Some(now);
        } else {
            state.cooldown_until = None;
        }
    }
    let slot = state.next_slot.filter(|s| *s > now).unwrap_or(now);
    let jitter = Duration::from_millis(rand::random::<u64>() % (jitter_ms + 1));
    state.next_slot = Some(slot + gap + jitter);
    Ok(slot - now)
}

/// Wait for this host's next slot (or fail fast while it is cooling down).
pub async fn acquire(url: &str) -> AppResult<()> {
    let key = host_key(url);
    let wait = {
        let mut hosts = HOSTS.lock().map_err(|e| AppError::Other(e.to_string()))?;
        let (gap, jitter) = pace_for(&key);
        reserve_paced(
            hosts.entry(key.clone()).or_default(),
            Instant::now(),
            gap,
            jitter,
            probe_every(&key),
        )
    };
    match wait {
        Ok(d) if d.is_zero() => Ok(()),
        Ok(d) => {
            tokio::time::sleep(d).await;
            Ok(())
        }
        Err(remaining) => Err(AppError::Site(format!(
            "{key} answered \"too many requests\" a moment ago, so ArcHive is pausing \
             requests to it (up to {}s; it retries every {}s and resumes as soon as the site \
             answers)",
            remaining.as_secs().max(1),
            probe_every(&key).as_secs()
        ))),
    }
}

/// A request to the host succeeded: end any cooldown.
pub fn note_ok(url: &str) {
    if let Ok(mut hosts) = HOSTS.lock() {
        if let Some(state) = hosts.get_mut(&host_key(url)) {
            if state.cooldown_until.take().is_some() {
                tracing::info!("[throttle] {} answers again; cooldown ended", host_key(url));
            }
            state.last_probe = None;
        }
    }
}

/// Whether ArcHive is backing off from this URL's host after a 429.
pub fn cooling_down(url: &str) -> bool {
    HOSTS
        .lock()
        .ok()
        .and_then(|h| h.get(&host_key(url)).and_then(|s| s.cooldown_until))
        .is_some_and(|until| until > Instant::now())
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
    fn slow_hosts_get_their_own_pace() {
        assert_eq!(pace_for("xpomelo.com").0, Duration::from_millis(6000));
        assert_eq!(pace_for("pornhub.com"), (MIN_GAP, JITTER_MS));
        let now = Instant::now();
        let mut s = HostState::default();
        let (gap, jitter) = pace_for("xpomelo.com");
        assert_eq!(
            reserve_paced(&mut s, now, gap, jitter, PROBE_EVERY),
            Ok(Duration::ZERO)
        );
        assert!(reserve_paced(&mut s, now, gap, jitter, PROBE_EVERY).unwrap() >= gap);
    }

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
        let max_gap = MIN_GAP + Duration::from_millis(JITTER_MS);
        assert_eq!(reserve(&mut s, now), Ok(Duration::ZERO));
        let second = reserve(&mut s, now).unwrap();
        assert!(second >= MIN_GAP && second <= max_gap, "{second:?}");
        let third = reserve(&mut s, now).unwrap();
        assert!(third >= second + MIN_GAP && third <= second + max_gap);
        // After the queue drains, a later request goes straight out.
        assert_eq!(reserve(&mut s, now + max_gap * 5), Ok(Duration::ZERO));

        let mut s = HostState::default();
        s.cooldown_until = Some(now + Duration::from_secs(120));
        // First request in a cooldown is a probe, the next ones wait.
        assert!(reserve(&mut s, now).is_ok());
        assert_eq!(
            reserve(&mut s, now + Duration::from_secs(1)),
            Err(Duration::from_secs(119))
        );
        // Another probe once PROBE_EVERY has passed.
        assert!(reserve(&mut s, now + PROBE_EVERY).is_ok());
        assert!(reserve(&mut s, now + PROBE_EVERY + Duration::from_secs(1)).is_err());
        assert!(reserve(&mut s, now + Duration::from_secs(121)).is_ok());
    }
}
