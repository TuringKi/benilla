//! The browser's local calendar and time zone, in the same shape as the native CRT adapter.
use js_sys::Date;

pub(super) struct Tm {
    pub tm_year: i32,
    pub tm_mon: i32,
    pub tm_mday: i32,
    pub tm_hour: i32,
    pub tm_min: i32,
    pub tm_sec: i32,
    pub tm_wday: i32,
    pub tm_yday: i32,
    pub tm_isdst: i32,
}

fn date(secs: i64) -> Date {
    Date::new(&((secs as f64 * 1000.0).into()))
}

fn seasonal(year: i64) -> [Date; 2] {
    [1, 7].map(|month| date(super::days_from_civil(year, month, 1) * 86_400))
}

fn standard_minutes(year: i64) -> f64 {
    let [jan, jul] = seasonal(year);
    jan.get_timezone_offset().max(jul.get_timezone_offset())
}

pub(super) fn localtime(secs: i64) -> Option<Tm> {
    let d = date(secs);
    if !d.get_time().is_finite() {
        return None;
    }
    let year = i64::from(d.get_full_year() as i32);
    let month = d.get_month() + 1;
    let day = d.get_date();
    Some(Tm {
        tm_year: year as i32 - 1900,
        tm_mon: month as i32 - 1,
        tm_mday: day as i32,
        tm_hour: d.get_hours() as i32,
        tm_min: d.get_minutes() as i32,
        tm_sec: d.get_seconds() as i32,
        tm_wday: d.get_day() as i32,
        tm_yday: (super::days_from_civil(year, month, day) - super::days_from_civil(year, 1, 1))
            as i32,
        tm_isdst: i32::from(d.get_timezone_offset() < standard_minutes(year)),
    })
}

pub(super) fn biases() -> (i32, i32) {
    let year = super::from_unix(super::unix_seconds()).year;
    let [jan, jul] = seasonal(year);
    let a = jan.get_timezone_offset();
    let b = jul.get_timezone_offset();
    let standard = a.max(b);
    (
        (standard * 60.0) as i32,
        ((a.min(b) - standard) * 60.0) as i32,
    )
}

pub(super) fn name(isdst: bool) -> Vec<u8> {
    let year = super::from_unix(super::unix_seconds()).year;
    let dates = seasonal(year);
    let standard = standard_minutes(year);
    let d = dates
        .iter()
        .find(|d| (d.get_timezone_offset() < standard) == isdst)
        .unwrap_or(&dates[0]);
    let text = String::from(d.to_time_string());
    if let Some((_, tail)) = text.split_once('(') {
        return tail.trim_end_matches(')').as_bytes().to_vec();
    }
    text.split_whitespace()
        .nth(1)
        .unwrap_or("")
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
pub(super) fn reread() {}
