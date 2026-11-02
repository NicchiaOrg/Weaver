//! The id of a request: the local time of its creation as `YYYY-MM-DDTHH-MM-SS`, with `-2`,
//! `-3`, … appended when that name is taken.

use std::cmp::Ordering;

/// A local date and time, as the clock of the developer's computer shows it. The core never reads
/// a clock: the caller says what time it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl LocalTime {
    /// Parses `YYYY-MM-DDTHH:MM:SS` (the time written with colons, as the interface sends it).
    pub fn parse(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 19
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
        {
            return None;
        }
        let number = |range: std::ops::Range<usize>| -> Option<u16> {
            let part = &b[range];
            part.iter()
                .all(u8::is_ascii_digit)
                .then(|| part.iter().fold(0u16, |n, d| n * 10 + u16::from(d - b'0')))
        };
        let time = LocalTime {
            year: number(0..4)?,
            month: number(5..7)? as u8,
            day: number(8..10)? as u8,
            hour: number(11..13)? as u8,
            minute: number(14..16)? as u8,
            second: number(17..19)? as u8,
        };
        let valid = (1..=12).contains(&time.month)
            && (1..=31).contains(&time.day)
            && time.hour < 24
            && time.minute < 60
            && time.second < 60;
        valid.then_some(time)
    }

    /// `YYYY-MM-DDTHH-MM-SS`.
    pub fn stem(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `YYYY-MM-DDTHH:MM:SS`, what the interface turns into a local time.
    pub fn iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// The id for a request created at `time`: its stem, or the stem with `-2`, `-3`, … when
/// `taken` says the name is used.
pub fn new_id(time: &LocalTime, taken: &dyn Fn(&str) -> bool) -> String {
    let stem = time.stem();
    if !taken(&stem) {
        return stem;
    }
    (2u32..)
        .map(|n| format!("{stem}-{n}"))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields a free name")
}

/// The creation time and the counter an id holds, `None` when `id` is not a request id (this
/// also keeps any path separator, dot or other name out of the requests folder).
pub fn parse_id(id: &str) -> Option<(LocalTime, u32)> {
    if id.len() < 19 || !id.is_char_boundary(19) {
        return None;
    }
    let (stamp, rest) = id.split_at(19);
    if !stamp.is_ascii() {
        return None;
    }
    let time = LocalTime::parse(&format!(
        "{}:{}:{}",
        &stamp[..13],
        &stamp[14..16],
        &stamp[17..]
    ))?;
    if stamp.as_bytes()[13] != b'-' || stamp.as_bytes()[16] != b'-' {
        return None;
    }
    let counter = match rest {
        "" => 1,
        _ => {
            let digits = rest.strip_prefix('-')?;
            if digits.is_empty()
                || digits.starts_with('0')
                || !digits.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            let n: u32 = digits.parse().ok()?;
            if n < 2 {
                return None;
            }
            n
        }
    };
    Some((time, counter))
}

/// Whether `id` is the id of a request.
pub fn is_request_id(id: &str) -> bool {
    parse_id(id).is_some()
}

/// Orders two request ids by creation: by time, then by the `-n` counter (numerically).
pub fn compare(a: &str, b: &str) -> Ordering {
    let key = |id: &str| parse_id(id).map(|(time, n)| (time.stem(), n));
    key(a).cmp(&key(b)).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(second: u8) -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 2,
            hour: 7,
            minute: 5,
            second,
        }
    }

    #[test]
    fn the_id_is_the_local_time_of_creation() {
        assert_eq!(time(9).stem(), "2026-11-02T07-05-09");
        assert_eq!(time(9).iso(), "2026-11-02T07:05:09");
        assert_eq!(new_id(&time(9), &|_| false), "2026-11-02T07-05-09");
    }

    #[test]
    fn a_taken_name_gets_the_next_counter() {
        let taken = [
            "2026-11-02T07-05-09",
            "2026-11-02T07-05-09-2",
            "2026-11-02T07-05-09-3",
        ];
        assert_eq!(
            new_id(&time(9), &|id| taken.contains(&id)),
            "2026-11-02T07-05-09-4"
        );
        assert_eq!(
            new_id(&time(9), &|id| id == "2026-11-02T07-05-09"),
            "2026-11-02T07-05-09-2"
        );
        assert_eq!(
            new_id(&time(10), &|id| taken.contains(&id)),
            "2026-11-02T07-05-10"
        );
    }

    #[test]
    fn a_time_is_parsed_with_colons_and_validated() {
        assert_eq!(LocalTime::parse("2026-11-02T07:05:09"), Some(time(9)));
        for bad in [
            "2026-11-02T07-05-09",
            "2026-13-02T07:05:09",
            "2026-11-00T07:05:09",
            "2026-11-02T24:05:09",
            "2026-11-02T07:60:09",
            "2026-11-02T07:05:60",
            "2026-11-02 07:05:09",
            "x",
            "",
        ] {
            assert_eq!(LocalTime::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn only_request_ids_name_files_of_the_requests_folder() {
        assert_eq!(parse_id("2026-11-02T07-05-09"), Some((time(9), 1)));
        assert_eq!(parse_id("2026-11-02T07-05-09-12"), Some((time(9), 12)));
        for bad in [
            "",
            "..",
            "../x",
            "2026-11-02T07-05-09/..",
            "2026-11-02T07:05:09",
            "2026-11-02T07-05-09-1",
            "2026-11-02T07-05-09-0",
            "2026-11-02T07-05-09-02",
            "2026-11-02T07-05-09-",
            "2026-11-02T07-05-09x",
            "2026-11-02T07-05-09.images",
            ".state",
            "aaaaaaaa",
        ] {
            assert!(!is_request_id(bad), "{bad}");
        }
        // Not a char boundary at 19: no panic.
        assert!(!is_request_id("2026-11-02T07-05-0é"));
    }

    #[test]
    fn newest_sorts_last_and_counters_compare_as_numbers() {
        let mut ids = vec![
            "2026-11-02T07-05-09-10",
            "2026-11-02T07-05-09",
            "2026-11-02T07-05-09-2",
            "2026-11-02T07-05-08",
            "2026-11-03T00-00-00",
        ];
        ids.sort_by(|a, b| compare(a, b));
        assert_eq!(
            ids,
            [
                "2026-11-02T07-05-08",
                "2026-11-02T07-05-09",
                "2026-11-02T07-05-09-2",
                "2026-11-02T07-05-09-10",
                "2026-11-03T00-00-00"
            ]
        );
    }
}
