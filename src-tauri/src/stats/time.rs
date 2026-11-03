//! Local-time helpers for statistics. Days, hours and months are the user's local ones,
//! and sessions that cross a boundary (e.g. midnight) are split across both sides.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bucket {
    Hour,
    Day,
    Month,
}

/// The first moment of a local calendar day. Where daylight saving starts at midnight,
/// midnight doesn't exist that day, so the first valid hour after it is used.
pub fn local_day_start(date: NaiveDate) -> DateTime<Local> {
    for hour in 0..24 {
        let candidate = NaiveTime::from_hms_opt(hour, 0, 0)
            .and_then(|time| Local.from_local_datetime(&date.and_time(time)).earliest());
        if let Some(start) = candidate {
            return start;
        }
    }
    Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN))
        .with_timezone(&Local)
}

/// The start of today in local time.
pub fn today_start() -> DateTime<Utc> {
    local_day_start(Local::now().date_naive()).with_timezone(&Utc)
}

/// The start of the local day `days` days before today.
pub fn days_ago_start(days: i64) -> DateTime<Utc> {
    local_day_start(Local::now().date_naive() - Duration::days(days)).with_timezone(&Utc)
}

/// Seconds of [start, end) that fall inside [from, to).
pub fn overlap_seconds(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> i64 {
    let lo = start.max(from);
    let hi = end.min(to);
    (hi - lo).num_seconds().max(0)
}

/// The local start of the bucket containing `t`.
pub fn bucket_start(t: DateTime<Local>, bucket: Bucket) -> DateTime<Local> {
    match bucket {
        Bucket::Hour => {
            t - Duration::minutes(t.minute() as i64)
                - Duration::seconds(t.second() as i64)
                - Duration::nanoseconds(t.nanosecond() as i64)
        }
        Bucket::Day => local_day_start(t.date_naive()),
        Bucket::Month => {
            local_day_start(NaiveDate::from_ymd_opt(t.year(), t.month(), 1).unwrap_or(t.date_naive()))
        }
    }
}

/// The local start of the bucket after the one containing `t`.
pub fn next_bucket_start(t: DateTime<Local>, bucket: Bucket) -> DateTime<Local> {
    match bucket {
        Bucket::Hour => bucket_start(t, bucket) + Duration::hours(1),
        Bucket::Day => match t.date_naive().succ_opt() {
            Some(next_day) => local_day_start(next_day),
            None => t + Duration::days(1),
        },
        Bucket::Month => {
            let (year, month) = if t.month() == 12 {
                (t.year() + 1, 1)
            } else {
                (t.year(), t.month() + 1)
            };
            NaiveDate::from_ymd_opt(year, month, 1)
                .map(local_day_start)
                .unwrap_or(t + Duration::days(31))
        }
    }
}

/// The chart label for the bucket containing `t`: "14:00", "2026-10-27" or "2026-10".
pub fn bucket_label(t: DateTime<Local>, bucket: Bucket) -> String {
    match bucket {
        Bucket::Hour => t.format("%H:00").to_string(),
        Bucket::Day => t.format("%Y-%m-%d").to_string(),
        Bucket::Month => t.format("%Y-%m").to_string(),
    }
}

/// Splits [start, end) at local bucket boundaries and calls `add` with the local start
/// of each piece and its length in seconds.
pub fn split_by_bucket(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    bucket: Bucket,
    mut add: impl FnMut(DateTime<Local>, i64),
) {
    let mut current = start;
    while current < end {
        let local = current.with_timezone(&Local);
        let boundary = next_bucket_start(local, bucket).with_timezone(&Utc);
        let piece_end = if boundary > current { boundary.min(end) } else { end };
        let seconds = (piece_end - current).num_seconds();
        if seconds > 0 {
            add(local, seconds);
        }
        current = piece_end;
    }
}

/// Labels of every bucket from the one containing `from` up to the one containing `to`,
/// so charts show empty periods as zero instead of skipping them.
pub fn bucket_labels(from: DateTime<Utc>, to: DateTime<Utc>, bucket: Bucket) -> Vec<String> {
    let mut labels = Vec::new();
    let end = to.with_timezone(&Local);
    let mut current = bucket_start(from.with_timezone(&Local), bucket);
    while current <= end {
        labels.push(bucket_label(current, bucket));
        let next = next_bucket_start(current, bucket);
        if next <= current {
            break;
        }
        current = next;
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_across_midnight() {
        let day = Local::now().date_naive() - Duration::days(3);
        let start = (local_day_start(day) + Duration::hours(23)).with_timezone(&Utc);
        let end = start + Duration::hours(2);
        let mut pieces = Vec::new();
        split_by_bucket(start, end, Bucket::Day, |t, s| pieces.push((bucket_label(t, Bucket::Day), s)));
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].1 + pieces[1].1, 7200);
        assert_ne!(pieces[0].0, pieces[1].0);
    }

    #[test]
    fn test_bucket_labels_fill_gaps() {
        let from = days_ago_start(6);
        let labels = bucket_labels(from, Utc::now(), Bucket::Day);
        assert_eq!(labels.len(), 7);
    }

    #[test]
    fn test_overlap_seconds() {
        let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
        let hour = Duration::hours(1);
        assert_eq!(overlap_seconds(t0, t0 + hour, t0 + Duration::minutes(30), t0 + hour * 2), 1800);
        assert_eq!(overlap_seconds(t0, t0 + hour, t0 + hour * 2, t0 + hour * 3), 0);
    }
}
