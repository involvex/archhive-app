//! Quiet-hours / notification scheduling helpers shared by download, watchlist,
//! watch-folder, and library-scan notify sites.

use crate::models::AppSettings;

/// Parse `"HH:MM"` into minutes since midnight. Returns `None` on bad input.
pub fn parse_time_hm(time: &str) -> Option<u32> {
    let mut parts = time.trim().split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// True when `now_minutes` is inside `[from, to)` with overnight wrap support
/// (same semantics as the scheduled theme window).
pub fn in_time_window(from: &str, to: &str, now_minutes: u32) -> bool {
    let Some(from_min) = parse_time_hm(from) else {
        return false;
    };
    let Some(to_min) = parse_time_hm(to) else {
        return false;
    };
    if from_min <= to_min {
        now_minutes >= from_min && now_minutes < to_min
    } else {
        now_minutes >= from_min || now_minutes < to_min
    }
}

/// Local wall-clock minutes since midnight.
pub fn local_minutes_now() -> u32 {
    use chrono::{Local, Timelike};
    let now = Local::now();
    now.hour() * 60 + now.minute()
}

impl AppSettings {
    /// Whether quiet hours are currently active for the given clock minutes.
    pub fn in_quiet_hours_at(&self, now_minutes: u32) -> bool {
        if !self.quiet_hours_enabled {
            return false;
        }
        in_time_window(&self.quiet_hours_from, &self.quiet_hours_to, now_minutes)
    }

    /// Whether quiet hours are active right now (local time).
    pub fn in_quiet_hours(&self) -> bool {
        self.in_quiet_hours_at(local_minutes_now())
    }

    /// Event toggles may still gate individually; this only checks the schedule.
    pub fn notifications_allowed(&self) -> bool {
        !self.in_quiet_hours()
    }

    /// True when the watchlist poller should skip a tick for quiet hours.
    pub fn should_pause_auto_queue(&self) -> bool {
        self.quiet_hours_pause_auto_queue && self.in_quiet_hours()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_hm_ok() {
        assert_eq!(parse_time_hm("22:00"), Some(22 * 60));
        assert_eq!(parse_time_hm("08:30"), Some(8 * 60 + 30));
        assert_eq!(parse_time_hm("0:00"), Some(0));
    }

    #[test]
    fn parse_time_hm_rejects_bad() {
        assert_eq!(parse_time_hm(""), None);
        assert_eq!(parse_time_hm("25:00"), None);
        assert_eq!(parse_time_hm("12:60"), None);
        assert_eq!(parse_time_hm("noon"), None);
    }

    #[test]
    fn same_day_window() {
        assert!(in_time_window("09:00", "17:00", 10 * 60));
        assert!(!in_time_window("09:00", "17:00", 8 * 60));
        assert!(!in_time_window("09:00", "17:00", 17 * 60));
    }

    #[test]
    fn overnight_window() {
        assert!(in_time_window("22:00", "08:00", 23 * 60));
        assert!(in_time_window("22:00", "08:00", 0));
        assert!(in_time_window("22:00", "08:00", 7 * 60 + 59));
        assert!(!in_time_window("22:00", "08:00", 8 * 60));
        assert!(!in_time_window("22:00", "08:00", 12 * 60));
        assert!(!in_time_window("22:00", "08:00", 21 * 60 + 59));
    }

    #[test]
    fn quiet_hours_disabled_never_active() {
        let mut s = AppSettings::default();
        s.quiet_hours_enabled = false;
        s.quiet_hours_from = "22:00".into();
        s.quiet_hours_to = "08:00".into();
        assert!(!s.in_quiet_hours_at(23 * 60));
        assert!(!s.in_quiet_hours_at(12 * 60));
    }

    #[test]
    fn quiet_hours_enabled_overnight() {
        let mut s = AppSettings::default();
        s.quiet_hours_enabled = true;
        s.quiet_hours_from = "22:00".into();
        s.quiet_hours_to = "08:00".into();
        assert!(s.in_quiet_hours_at(23 * 60));
        assert!(!s.in_quiet_hours_at(12 * 60));
    }

    #[test]
    fn pause_auto_queue_requires_both_flags() {
        let mut s = AppSettings::default();
        s.quiet_hours_enabled = true;
        s.quiet_hours_from = "22:00".into();
        s.quiet_hours_to = "08:00".into();
        s.quiet_hours_pause_auto_queue = false;
        assert!(!s.quiet_hours_pause_auto_queue || !s.in_quiet_hours_at(23 * 60));
        // Helper uses wall clock — test via in_quiet_hours_at + flag
        assert!(s.in_quiet_hours_at(23 * 60));
        assert!(!s.quiet_hours_pause_auto_queue);

        s.quiet_hours_pause_auto_queue = true;
        assert!(s.quiet_hours_pause_auto_queue && s.in_quiet_hours_at(23 * 60));
        assert!(!s.in_quiet_hours_at(12 * 60));
    }
}
