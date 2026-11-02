//! Instants: UTC ISO 8601 with milliseconds (`2026-11-02T07:07:19.522Z`), from an injected clock.
//!
//! Every instant written into a project file comes from a [`Clock`] at the moment of the event;
//! an epoch or any other placeholder is refused ([`now_instant`]).

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A source of the current time, injected so that tests control it.
pub trait Clock: Send + Sync {
    /// Milliseconds since 1970-01-01T00:00:00Z.
    fn now_ms(&self) -> i64;
}

/// The operating system's clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => i64::try_from(d.as_millis()).unwrap_or(i64::MAX),
            Err(e) => -i64::try_from(e.duration().as_millis()).unwrap_or(i64::MAX),
        }
    }
}

/// A clock moved by hand: it returns the same time until [`ManualClock::set`] or
/// [`ManualClock::advance`] changes it.
#[derive(Debug, Default)]
pub struct ManualClock(AtomicI64);

impl ManualClock {
    pub fn new(ms: i64) -> Self {
        Self(AtomicI64::new(ms))
    }

    /// A clock set at the given instant text (`2026-11-02T07:07:19.522Z`).
    pub fn at(instant: &str) -> Self {
        Self::new(parse_instant(instant).expect("a valid instant"))
    }

    pub fn set(&self, ms: i64) {
        self.0.store(ms, Ordering::SeqCst);
    }

    pub fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the clock returned {0} ms, which is not a real instant")]
pub struct PlaceholderInstant(pub i64);

/// The current instant of `clock`, formatted. A time at or before the epoch is a placeholder
/// and is refused.
pub fn now_instant(clock: &dyn Clock) -> Result<String, PlaceholderInstant> {
    let ms = clock.now_ms();
    if ms <= 0 {
        return Err(PlaceholderInstant(ms));
    }
    Ok(format_instant(ms))
}

/// Formats milliseconds since the epoch as `YYYY-MM-DDTHH:MM:SS.mmmZ` (UTC).
pub fn format_instant(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let in_day = ms.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second, milli) = (
        in_day / 3_600_000,
        in_day / 60_000 % 60,
        in_day / 1000 % 60,
        in_day % 1000,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// Parses the exact format of [`format_instant`]; anything else is `None`.
pub fn parse_instant(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 24 {
        return None;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = &b[range];
        if part.iter().all(u8::is_ascii_digit) {
            Some(part.iter().fold(0i64, |n, d| n * 10 + i64::from(d - b'0')))
        } else {
            None
        }
    };
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'.'
        || b[23] != b'Z'
    {
        return None;
    }
    let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let (hour, minute, second, milli) = (
        digits(11..13)?,
        digits(14..16)?,
        digits(17..19)?,
        digits(20..23)?,
    );
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000 + milli)
}

/// Whether `text` is an instant in the exact format of [`format_instant`].
pub fn is_instant(text: &str) -> bool {
    parse_instant(text).is_some()
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

// Howard Hinnant's civil-date algorithms (proleptic Gregorian calendar).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instants() {
        assert_eq!(format_instant(1), "1970-01-01T00:00:00.001Z");
        assert_eq!(
            format_instant(1_793_603_239_522),
            "2026-11-02T07:07:19.522Z"
        );
        assert_eq!(format_instant(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn parse_is_the_inverse_of_format() {
        for ms in [
            1,
            999,
            86_400_000,
            951_782_400_123,
            1_793_603_239_522,
            4_102_444_799_999,
        ] {
            assert_eq!(parse_instant(&format_instant(ms)), Some(ms));
        }
    }

    #[test]
    fn rejects_other_shapes() {
        for text in [
            "2026-11-02T07:07:19Z",
            "2026-11-02 07:07:19.522Z",
            "2026-11-02T07:07:19.522+00:00",
            "2026-13-02T07:07:19.522Z",
            "2026-02-30T07:07:19.522Z",
            "2026-11-02T24:07:19.522Z",
            "2026-11-02T07:07:19.52Z",
            "",
        ] {
            assert!(!is_instant(text), "{text}");
        }
    }

    #[test]
    fn the_injected_clock_gives_the_instant_and_the_epoch_is_refused() {
        let clock = ManualClock::at("2026-11-02T07:07:19.522Z");
        assert_eq!(now_instant(&clock).unwrap(), "2026-11-02T07:07:19.522Z");
        clock.advance(478);
        assert_eq!(now_instant(&clock).unwrap(), "2026-11-02T07:07:20.000Z");
        assert_eq!(
            now_instant(&ManualClock::new(0)),
            Err(PlaceholderInstant(0))
        );
        assert!(now_instant(&ManualClock::new(-5)).is_err());
    }

    #[test]
    fn the_system_clock_is_a_real_instant() {
        assert!(is_instant(&now_instant(&SystemClock).unwrap()));
    }
}
