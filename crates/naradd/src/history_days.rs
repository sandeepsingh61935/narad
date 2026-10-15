//! Local calendar bounds for history filters and stats (FR-HIS-3, FR-HIS-8).

/// Bounds for `created_at`. `to_ms` is the next local midnight and is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DayWindow {
    Range {
        from_ms: Option<i64>,
        to_ms: Option<i64>,
    },
    /// `from_day` is after `to_day`. The search matches nothing.
    Empty,
}

/// Interpret `YYYY-MM-DD` at a fixed UTC offset in seconds east of UTC.
///
/// A blank side is open. The end is the local midnight after `to_day`.
pub(crate) fn history_day_window_with_offset(
    from_day: &str,
    to_day: &str,
    offset_secs: i32,
) -> Result<DayWindow, String> {
    let from = parse_day(from_day)?;
    let to = parse_day(to_day)?;
    if let (Some(from), Some(to)) = (from, to) {
        if civil_ord(from) > civil_ord(to) {
            return Ok(DayWindow::Empty);
        }
    }
    Ok(DayWindow::Range {
        from_ms: from.map(|day| midnight_ms(day, offset_secs)),
        to_ms: to.map(|day| midnight_ms(day, offset_secs) + 86_400_000),
    })
}

fn midnight_ms(day: Civil, offset_secs: i32) -> i64 {
    days_from_civil(day.0, day.1, day.2) * 86_400_000 - i64::from(offset_secs) * 1_000
}

type Civil = (i32, u32, u32);

fn civil_ord(day: Civil) -> i64 {
    days_from_civil(day.0, day.1, day.2)
}

fn parse_day(raw: &str) -> Result<Option<Civil>, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let mut parts = text.split('-');
    let year = parts.next();
    let month = parts.next();
    let day = parts.next();
    let extra = parts.next();
    let (Some(year), Some(month), Some(day), None) = (year, month, day, extra) else {
        return Err(format!("invalid date: {text}"));
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return Err(format!("invalid date: {text}"));
    }
    let year: i32 = year.parse().map_err(|_| format!("invalid date: {text}"))?;
    let month: u32 = month.parse().map_err(|_| format!("invalid date: {text}"))?;
    let day: u32 = day.parse().map_err(|_| format!("invalid date: {text}"))?;
    if month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) {
        return Err(format!("invalid date: {text}"));
    }
    Ok(Some((year, month, day)))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Trimmed text filters plus the day window at a fixed offset.
pub(crate) struct PreparedHistory {
    pub filter: narad_core::store::HistoryFilter,
    pub match_none: bool,
}

/// Same as [`history_filter_at_offset`], using the machine's local timezone.
pub(crate) fn history_filter(
    from_day: &str,
    to_day: &str,
    mode: &str,
    app_id: &str,
    src_lang: &str,
) -> Result<PreparedHistory, String> {
    let from = parse_day(from_day)?;
    let to = parse_day(to_day)?;
    let filter = narad_core::store::HistoryFilter {
        mode: mode.trim().to_string(),
        app_id: app_id.trim().to_string(),
        src_lang: src_lang.trim().to_string(),
        ..narad_core::store::HistoryFilter::default()
    };
    if let (Some(from), Some(to)) = (from, to) {
        if civil_ord(from) > civil_ord(to) {
            return Ok(PreparedHistory {
                filter,
                match_none: true,
            });
        }
    }
    let mut filter = filter;
    filter.from_ms = match from {
        Some(day) => Some(local_midnight_ms(day)?),
        None => None,
    };
    filter.to_ms = match to {
        Some(day) => Some(local_midnight_ms(next_civil(day))?),
        None => None,
    };
    Ok(PreparedHistory {
        filter,
        match_none: false,
    })
}

fn next_civil(day: Civil) -> Civil {
    let (mut year, mut month, mut dom) = day;
    dom += 1;
    if dom > days_in_month(year, month) {
        dom = 1;
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }
    (year, month, dom)
}

fn local_midnight_ms(day: Civil) -> Result<i64, String> {
    let mut tm = LibcTm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: day.2 as i32,
        tm_mon: day.1 as i32 - 1,
        tm_year: day.0 - 1900,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: -1,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    let secs = unsafe { mktime(&mut tm) };
    if secs == -1 {
        return Err(format!(
            "invalid date: {:04}-{:02}-{:02}",
            day.0, day.1, day.2
        ));
    }
    Ok(secs.saturating_mul(1000))
}

#[repr(C)]
struct LibcTm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const i8,
}

unsafe extern "C" {
    fn mktime(tm: *mut LibcTm) -> i64;
    fn localtime_r(timep: *const i64, result: *mut LibcTm) -> *mut LibcTm;
}

/// Inclusive `from_day` and `to_day`, and the millisecond window the store receives.
pub(crate) struct StatsBounds {
    pub from_ms: i64,
    pub to_ms: i64,
    pub from_day: String,
    pub to_day: String,
}

/// `day` or `week` at a fixed UTC offset. Week is Monday through Sunday.
pub(crate) fn stats_window_with_offset(
    range: &str,
    anchor_day: &str,
    offset_secs: i32,
) -> Result<DayWindow, String> {
    let anchor = match parse_day(anchor_day)? {
        Some(day) => day,
        None => return Err(format!("invalid date: {}", anchor_day.trim())),
    };
    let (from_day, to_day) = stats_days(range, anchor)?;
    history_day_window_with_offset(&format_civil(from_day), &format_civil(to_day), offset_secs)
}

/// Live day or Monday-week bounds. The store sees only the millisecond window.
pub(crate) fn stats_bounds(range: &str) -> Result<StatsBounds, String> {
    let today = local_today()?;
    let (from_day, to_day) = stats_days(range, today)?;
    let from_day = format_civil(from_day);
    let to_day = format_civil(to_day);
    let prepared = history_filter(&from_day, &to_day, "", "", "")?;
    match (prepared.filter.from_ms, prepared.filter.to_ms) {
        (Some(from_ms), Some(to_ms)) => Ok(StatsBounds {
            from_ms,
            to_ms,
            from_day,
            to_day,
        }),
        _ => Err("missing stats bounds".into()),
    }
}

fn stats_days(range: &str, anchor: Civil) -> Result<(Civil, Civil), String> {
    match range.trim() {
        "day" => Ok((anchor, anchor)),
        "week" => Ok(monday_through_sunday(anchor)),
        other => Err(format!("unknown stats range: {other}")),
    }
}

fn format_civil(day: Civil) -> String {
    format!("{:04}-{:02}-{:02}", day.0, day.1, day.2)
}

/// `1970-01-01` is Thursday, so Monday is weekday 0 when this is 0.
fn days_since_monday(day: Civil) -> i64 {
    (civil_ord(day) + 3).rem_euclid(7)
}

fn prev_civil(day: Civil) -> Civil {
    let (year, month, dom) = day;
    if dom > 1 {
        return (year, month, dom - 1);
    }
    if month == 1 {
        return (year - 1, 12, 31);
    }
    let month = month - 1;
    (year, month, days_in_month(year, month))
}

fn monday_through_sunday(day: Civil) -> (Civil, Civil) {
    let mut monday = day;
    for _ in 0..days_since_monday(day) {
        monday = prev_civil(monday);
    }
    let mut sunday = monday;
    for _ in 0..6 {
        sunday = next_civil(sunday);
    }
    (monday, sunday)
}

fn local_today() -> Result<Civil, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "system time before Unix epoch".to_string())?
        .as_millis();
    let now_ms = i64::try_from(now_ms).map_err(|_| "system time overflow".to_string())?;
    let secs = now_ms.div_euclid(1_000);
    let mut tm = LibcTm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    let ptr = unsafe { localtime_r(&secs, &mut tm) };
    if ptr.is_null() {
        return Err("local time unavailable".into());
    }
    let month = tm.tm_mon + 1;
    let day = tm.tm_mday;
    if month < 1 || day < 1 {
        return Err("local time unavailable".into());
    }
    let guess = (tm.tm_year + 1900, month as u32, day as u32);
    for candidate in [guess, prev_civil(guess), next_civil(guess)] {
        let start = local_midnight_ms(candidate)?;
        let end = local_midnight_ms(next_civil(candidate))?;
        if now_ms >= start && now_ms < end {
            return Ok(candidate);
        }
    }
    Err("local time unavailable".into())
}

pub(crate) fn history_filter_at_offset(
    from_day: &str,
    to_day: &str,
    mode: &str,
    app_id: &str,
    src_lang: &str,
    offset_secs: i32,
) -> Result<PreparedHistory, String> {
    let window = history_day_window_with_offset(from_day, to_day, offset_secs)?;
    let mut filter = narad_core::store::HistoryFilter {
        mode: mode.trim().to_string(),
        app_id: app_id.trim().to_string(),
        src_lang: src_lang.trim().to_string(),
        ..narad_core::store::HistoryFilter::default()
    };
    match window {
        DayWindow::Empty => Ok(PreparedHistory {
            filter,
            match_none: true,
        }),
        DayWindow::Range { from_ms, to_ms } => {
            filter.from_ms = from_ms;
            filter.to_ms = to_ms;
            Ok(PreparedHistory {
                filter,
                match_none: false,
            })
        }
    }
}

/// Days since Unix epoch for a Gregorian date. `1970-01-01` is 0.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = (year - era * 400) as u64;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * month_index as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_day_is_local_midnight_through_the_next_midnight() {
        let window = history_day_window_with_offset("1970-01-02", "1970-01-02", 0).expect("window");
        assert_eq!(
            window,
            DayWindow::Range {
                from_ms: Some(86_400_000),
                to_ms: Some(172_800_000),
            }
        );
    }

    #[test]
    fn the_offset_shifts_local_midnight() {
        let window =
            history_day_window_with_offset("1970-01-02", "1970-01-02", 19_800).expect("window");
        assert_eq!(
            window,
            DayWindow::Range {
                from_ms: Some(66_600_000),
                to_ms: Some(153_000_000),
            }
        );
    }

    #[test]
    fn a_blank_side_stays_open_and_whitespace_is_blank() {
        let from_only = history_day_window_with_offset("1970-01-02", "  ", 0).expect("from");
        assert_eq!(
            from_only,
            DayWindow::Range {
                from_ms: Some(86_400_000),
                to_ms: None,
            }
        );
        let to_only = history_day_window_with_offset("", "1970-01-02", 0).expect("to");
        assert_eq!(
            to_only,
            DayWindow::Range {
                from_ms: None,
                to_ms: Some(172_800_000),
            }
        );
    }

    #[test]
    fn a_reversed_range_is_empty_and_a_bad_date_is_an_error() {
        let reversed = history_day_window_with_offset("1970-01-03", "1970-01-02", 0).expect("rev");
        assert_eq!(reversed, DayWindow::Empty);
        let err = history_day_window_with_offset("1970-13-01", "", 0).expect_err("month");
        assert!(err.contains("1970-13-01"));
        assert!(history_day_window_with_offset("1970-02-29", "", 0).is_err());
        let leap = history_day_window_with_offset("1972-02-29", "1972-02-29", 0).expect("leap");
        assert_eq!(
            leap,
            DayWindow::Range {
                from_ms: Some(68_169_600_000),
                to_ms: Some(68_256_000_000),
            }
        );
    }

    #[test]
    fn history_args_trim_and_a_reversed_range_matches_nothing() {
        let prepared =
            history_filter_at_offset("1970-01-02", "1970-01-02", " email ", "  ", " en ", 0)
                .expect("prepared");
        assert!(!prepared.match_none);
        assert_eq!(prepared.filter.mode, "email");
        assert_eq!(prepared.filter.app_id, "");
        assert_eq!(prepared.filter.src_lang, "en");
        assert_eq!(prepared.filter.from_ms, Some(86_400_000));
        assert_eq!(prepared.filter.to_ms, Some(172_800_000));

        let reversed =
            history_filter_at_offset("1970-01-03", "1970-01-02", "", "", "", 0).expect("reversed");
        assert!(reversed.match_none);
        assert!(history_filter_at_offset("1970-13-01", "", "email", "", "", 0).is_err());
    }

    #[test]
    fn stats_window_is_the_civil_day_or_the_monday_week() {
        let week = history_day_window_with_offset("2026-10-12", "2026-10-18", 0).expect("week");
        for anchor in ["2026-10-15", "2026-10-18", "2026-10-12"] {
            let window = stats_window_with_offset("week", anchor, 0).expect(anchor);
            assert_eq!(window, week);
        }
        let day = history_day_window_with_offset("2026-10-15", "2026-10-15", 0).expect("day");
        assert_eq!(
            stats_window_with_offset("day", "2026-10-15", 0).expect("day window"),
            day
        );
        let err = stats_window_with_offset("year", "2026-10-15", 0).expect_err("year");
        assert!(err.contains("unknown stats range: year"));
    }

    #[test]
    fn a_local_day_spans_twenty_four_hours() {
        let prepared = history_filter("1970-01-02", "1970-01-02", "", "", "").expect("local");
        let from = prepared.filter.from_ms.expect("from");
        let to = prepared.filter.to_ms.expect("to");
        assert_eq!(to - from, 86_400_000);
        assert!(!prepared.match_none);
    }
}
