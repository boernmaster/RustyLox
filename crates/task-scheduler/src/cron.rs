//! Helpers for building and describing 6-field cron expressions.
//!
//! The `cron` crate used by the task scheduler requires **6 fields**:
//! ```text
//! {second} {minute} {hour} {day-of-month} {month} {day-of-week}
//! ```
//! This module provides [`Cron`] presets and a fluent [`CronBuilder`] so callers
//! never have to memorise or hand-write raw cron strings.
//!
//! # Quick examples
//! ```rust
//! use task_scheduler::cron::{Cron, Weekday};
//!
//! let daily_2am  = Cron::daily_at(2, 0);           // "0 0 2 * * *"
//! let every_6h   = Cron::every_n_hours(6);          // "0 0 */6 * * *"
//! let sunday_mid = Cron::weekly_on(Weekday::Sunday, 0, 0); // "0 0 0 * * 0"
//!
//! // Fluent builder for anything more custom
//! let expr = Cron::builder()
//!     .at_hour(9)
//!     .at_minute(30)
//!     .on_weekdays(&[Weekday::Monday, Weekday::Friday])
//!     .build();
//! assert_eq!(expr.as_str(), "0 30 9 * * 1,5");
//! ```

use cron::Schedule;
use std::str::FromStr;

// ── Weekday ─────────────────────────────────────────────────────────────────

/// Day of the week for use in cron expressions (Sunday = 0 … Saturday = 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl Weekday {
    fn num(self) -> u8 {
        self as u8
    }
}

impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Weekday::Sunday => "Sunday",
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
        };
        f.write_str(name)
    }
}

// ── CronExpr ─────────────────────────────────────────────────────────────────

/// A validated 6-field cron expression string.
///
/// Obtain one via [`Cron`] presets, [`CronBuilder`], or [`CronExpr::parse`].
/// The inner string is always valid and can be fed directly to the `cron` crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr(String);

impl CronExpr {
    /// Returns the raw cron string, e.g. `"0 0 2 * * *"`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parse and validate a raw cron string.
    ///
    /// Returns `Err` with a human-readable message if the expression is invalid.
    pub fn parse(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();
        Schedule::from_str(&s).map_err(|e| format!("Invalid cron expression '{}': {}", s, e))?;
        Ok(Self(s))
    }

    /// Returns a human-readable description of the schedule.
    pub fn describe(&self) -> String {
        describe_cron(&self.0)
    }
}

impl std::fmt::Display for CronExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<CronExpr> for String {
    fn from(e: CronExpr) -> String {
        e.0
    }
}

impl AsRef<str> for CronExpr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ── Cron presets ─────────────────────────────────────────────────────────────

/// Factory for common cron schedule presets and the fluent [`CronBuilder`].
///
/// All produced expressions use the 6-field format required by the `cron` crate:
/// `{second} {minute} {hour} {day-of-month} {month} {day-of-week}`.
pub struct Cron;

impl Cron {
    // ── Sub-minute ────────────────────────────────────────────────────────────

    /// Every minute at second 0: `0 * * * * *`
    pub fn every_minute() -> CronExpr {
        CronExpr("0 * * * * *".into())
    }

    /// Every `n` minutes at second 0: `0 */n * * * *`
    ///
    /// `n` must be in the range 1–59.
    pub fn every_n_minutes(n: u32) -> CronExpr {
        assert!((1..=59).contains(&n), "n must be between 1 and 59");
        CronExpr(format!("0 */{} * * * *", n))
    }

    // ── Hourly ────────────────────────────────────────────────────────────────

    /// Once per hour at minute 0: `0 0 * * * *`
    pub fn hourly() -> CronExpr {
        CronExpr("0 0 * * * *".into())
    }

    /// Once per hour at a specific minute: `0 {minute} * * * *`
    pub fn hourly_at(minute: u32) -> CronExpr {
        assert!(minute < 60, "minute must be 0–59");
        CronExpr(format!("0 {} * * * *", minute))
    }

    /// Every `n` hours at minute 0: `0 0 */n * * *`
    ///
    /// `n` must be in the range 1–23.
    pub fn every_n_hours(n: u32) -> CronExpr {
        assert!((1..=23).contains(&n), "n must be between 1 and 23");
        CronExpr(format!("0 0 */{} * * *", n))
    }

    // ── Daily ─────────────────────────────────────────────────────────────────

    /// Once a day at `hour:minute`: `0 {minute} {hour} * * *`
    pub fn daily_at(hour: u32, minute: u32) -> CronExpr {
        assert!(hour < 24, "hour must be 0–23");
        assert!(minute < 60, "minute must be 0–59");
        CronExpr(format!("0 {} {} * * *", minute, hour))
    }

    // ── Weekly ────────────────────────────────────────────────────────────────

    /// Once a week on `day` at `hour:minute`: `0 {minute} {hour} * * {dow}`
    pub fn weekly_on(day: Weekday, hour: u32, minute: u32) -> CronExpr {
        assert!(hour < 24, "hour must be 0–23");
        assert!(minute < 60, "minute must be 0–59");
        CronExpr(format!("0 {} {} * * {}", minute, hour, day.num()))
    }

    // ── Monthly ───────────────────────────────────────────────────────────────

    /// Once a month on day `dom` at `hour:minute`: `0 {minute} {hour} {dom} * *`
    ///
    /// `dom` must be in the range 1–31.
    pub fn monthly_on(dom: u32, hour: u32, minute: u32) -> CronExpr {
        assert!((1..=31).contains(&dom), "day-of-month must be 1–31");
        assert!(hour < 24, "hour must be 0–23");
        assert!(minute < 60, "minute must be 0–59");
        CronExpr(format!("0 {} {} {} * *", minute, hour, dom))
    }

    // ── Builder ───────────────────────────────────────────────────────────────

    /// Start building a custom cron expression with the fluent [`CronBuilder`].
    ///
    /// Defaults: second = 0, all other fields = `*` (i.e. every minute).
    pub fn builder() -> CronBuilder {
        CronBuilder::default()
    }
}

// ── CronBuilder ──────────────────────────────────────────────────────────────

/// Fluent builder for custom 6-field cron expressions.
///
/// Start with [`Cron::builder()`]. Every setter returns `Self` so calls can
/// be chained. Finish with [`CronBuilder::build`].
///
/// # Example
/// ```rust
/// use task_scheduler::cron::{Cron, Weekday};
///
/// // Weekdays at 09:30
/// let expr = Cron::builder()
///     .at_hour(9)
///     .at_minute(30)
///     .on_weekdays(&[Weekday::Monday, Weekday::Tuesday,
///                    Weekday::Wednesday, Weekday::Thursday, Weekday::Friday])
///     .build();
/// assert_eq!(expr.as_str(), "0 30 9 * * 1,2,3,4,5");
/// ```
#[derive(Debug, Clone)]
pub struct CronBuilder {
    second: Field,
    minute: Field,
    hour: Field,
    day_of_month: Field,
    month: Field,
    day_of_week: Field,
}

impl Default for CronBuilder {
    /// Defaults to second = 0, all other fields = `*` (fires every minute).
    fn default() -> Self {
        Self {
            second: Field::Value(0),
            minute: Field::Any,
            hour: Field::Any,
            day_of_month: Field::Any,
            month: Field::Any,
            day_of_week: Field::Any,
        }
    }
}

impl CronBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Second ────────────────────────────────────────────────────────────────

    /// Fire at a specific second (0–59). Default is 0.
    pub fn at_second(mut self, sec: u32) -> Self {
        self.second = Field::Value(sec);
        self
    }

    // ── Minute ────────────────────────────────────────────────────────────────

    /// Fire at a specific minute (0–59).
    pub fn at_minute(mut self, min: u32) -> Self {
        self.minute = Field::Value(min);
        self
    }

    /// Fire every `n` minutes (`*/n`). `n` must be 1–59.
    pub fn every_n_minutes(mut self, n: u32) -> Self {
        self.minute = Field::Step(n);
        self
    }

    // ── Hour ──────────────────────────────────────────────────────────────────

    /// Fire at a specific hour (0–23).
    pub fn at_hour(mut self, hour: u32) -> Self {
        self.hour = Field::Value(hour);
        self
    }

    /// Fire every `n` hours (`*/n`). `n` must be 1–23.
    pub fn every_n_hours(mut self, n: u32) -> Self {
        self.hour = Field::Step(n);
        self
    }

    // ── Day of month ──────────────────────────────────────────────────────────

    /// Fire on a specific day of the month (1–31).
    pub fn on_day_of_month(mut self, day: u32) -> Self {
        self.day_of_month = Field::Value(day);
        self
    }

    // ── Month ─────────────────────────────────────────────────────────────────

    /// Fire only in a specific month (1 = January … 12 = December).
    pub fn in_month(mut self, month: u32) -> Self {
        self.month = Field::Value(month);
        self
    }

    // ── Day of week ───────────────────────────────────────────────────────────

    /// Fire on a single weekday.
    pub fn on_weekday(mut self, day: Weekday) -> Self {
        self.day_of_week = Field::List(vec![day.num()]);
        self
    }

    /// Fire on multiple weekdays (order does not matter).
    pub fn on_weekdays(mut self, days: &[Weekday]) -> Self {
        let mut nums: Vec<u8> = days.iter().map(|d| d.num()).collect();
        nums.sort_unstable();
        nums.dedup();
        self.day_of_week = Field::List(nums);
        self
    }

    /// Build the [`CronExpr`].
    ///
    /// # Panics
    /// Panics in debug builds if the resulting expression fails to parse.
    pub fn build(self) -> CronExpr {
        let expr = format!(
            "{} {} {} {} {} {}",
            self.second, self.minute, self.hour, self.day_of_month, self.month, self.day_of_week,
        );
        debug_assert!(
            Schedule::from_str(&expr).is_ok(),
            "CronBuilder produced invalid expression: {}",
            expr
        );
        CronExpr(expr)
    }
}

// ── Internal field representation ────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Field {
    /// `*`  — match any value
    Any,
    /// A single numeric value
    Value(u32),
    /// `*/n` — every n units
    Step(u32),
    /// `a,b,c` — explicit list (used for weekdays)
    List(Vec<u8>),
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Any => f.write_str("*"),
            Field::Value(v) => write!(f, "{}", v),
            Field::Step(n) => write!(f, "*/{}", n),
            Field::List(vs) => {
                let s: Vec<String> = vs.iter().map(|v| v.to_string()).collect();
                f.write_str(&s.join(","))
            }
        }
    }
}

// ── Human-readable description ────────────────────────────────────────────────

/// Translate a 6-field cron expression into a short English description.
///
/// This is a best-effort translation for common patterns; complex expressions
/// fall back to showing the raw string.
pub fn describe_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 6 {
        return format!("custom schedule: {}", expr);
    }
    let (sec, min, hour, dom, month, dow) = (
        fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
    );

    // Every minute
    if min == "*" && hour == "*" && dom == "*" && month == "*" && dow == "*" {
        return "every minute".into();
    }

    // Every N minutes
    if let Some(n) = step_value(min) {
        if hour == "*" && dom == "*" && month == "*" && dow == "*" {
            return format!("every {} minutes", n);
        }
    }

    // Hourly / every N hours
    if min == "0" && dom == "*" && month == "*" && dow == "*" {
        if hour == "*" {
            return "every hour".into();
        }
        if let Some(n) = step_value(hour) {
            return format!("every {} hours", n);
        }
    }

    // Daily
    if dom == "*" && month == "*" && dow == "*" {
        if let (Ok(h), Ok(m)) = (hour.parse::<u32>(), min.parse::<u32>()) {
            return format!("daily at {:02}:{:02}", h, m);
        }
    }

    // Weekly
    if dom == "*" && month == "*" {
        if let (Ok(d), Ok(h), Ok(m)) = (dow.parse::<u32>(), hour.parse::<u32>(), min.parse::<u32>())
        {
            let day_name = weekday_name(d);
            return format!("every {} at {:02}:{:02}", day_name, h, m);
        }
        // Multiple weekdays
        if dow.contains(',') {
            if let (Ok(h), Ok(m)) = (hour.parse::<u32>(), min.parse::<u32>()) {
                return format!(
                    "on {} at {:02}:{:02}",
                    dow.split(',')
                        .filter_map(|d| d.parse::<u32>().ok())
                        .map(weekday_name)
                        .collect::<Vec<_>>()
                        .join(", "),
                    h,
                    m
                );
            }
        }
    }

    // Monthly
    if month == "*" && dow == "*" {
        if let (Ok(d), Ok(h), Ok(m)) = (dom.parse::<u32>(), hour.parse::<u32>(), min.parse::<u32>())
        {
            return format!("on day {} of every month at {:02}:{:02}", d, h, m);
        }
    }

    // Fallback
    let _ = sec; // silence unused warning
    format!("custom schedule: {}", expr)
}

fn step_value(field: &str) -> Option<u32> {
    field.strip_prefix("*/").and_then(|n| n.parse::<u32>().ok())
}

fn weekday_name(n: u32) -> String {
    match n {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "?",
    }
    .into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Preset expressions ────────────────────────────────────────────────────

    #[test]
    fn every_minute() {
        let e = Cron::every_minute();
        assert_eq!(e.as_str(), "0 * * * * *");
    }

    #[test]
    fn every_n_minutes() {
        assert_eq!(Cron::every_n_minutes(15).as_str(), "0 */15 * * * *");
        assert_eq!(Cron::every_n_minutes(1).as_str(), "0 */1 * * * *");
    }

    #[test]
    fn hourly() {
        assert_eq!(Cron::hourly().as_str(), "0 0 * * * *");
    }

    #[test]
    fn hourly_at() {
        assert_eq!(Cron::hourly_at(30).as_str(), "0 30 * * * *");
    }

    #[test]
    fn every_n_hours() {
        assert_eq!(Cron::every_n_hours(6).as_str(), "0 0 */6 * * *");
    }

    #[test]
    fn daily_at() {
        assert_eq!(Cron::daily_at(2, 0).as_str(), "0 0 2 * * *");
        assert_eq!(Cron::daily_at(14, 30).as_str(), "0 30 14 * * *");
    }

    #[test]
    fn weekly_on() {
        assert_eq!(
            Cron::weekly_on(Weekday::Sunday, 0, 0).as_str(),
            "0 0 0 * * 0"
        );
        assert_eq!(
            Cron::weekly_on(Weekday::Monday, 9, 15).as_str(),
            "0 15 9 * * 1"
        );
    }

    #[test]
    fn monthly_on() {
        assert_eq!(Cron::monthly_on(1, 3, 0).as_str(), "0 0 3 1 * *");
    }

    // ── Builder ───────────────────────────────────────────────────────────────

    #[test]
    fn builder_defaults() {
        // Default = fires every minute at second 0
        let e = Cron::builder().build();
        assert_eq!(e.as_str(), "0 * * * * *");
    }

    #[test]
    fn builder_daily() {
        let e = Cron::builder().at_hour(2).at_minute(0).build();
        assert_eq!(e.as_str(), "0 0 2 * * *");
    }

    #[test]
    fn builder_weekdays() {
        let e = Cron::builder()
            .at_hour(9)
            .at_minute(30)
            .on_weekdays(&[Weekday::Monday, Weekday::Friday])
            .build();
        assert_eq!(e.as_str(), "0 30 9 * * 1,5");
    }

    #[test]
    fn builder_every_n_minutes() {
        let e = Cron::builder().every_n_minutes(10).build();
        assert_eq!(e.as_str(), "0 */10 * * * *");
    }

    #[test]
    fn builder_every_n_hours() {
        let e = Cron::builder().at_minute(0).every_n_hours(4).build();
        assert_eq!(e.as_str(), "0 0 */4 * * *");
    }

    #[test]
    fn builder_monthly() {
        let e = Cron::builder()
            .on_day_of_month(15)
            .at_hour(8)
            .at_minute(0)
            .build();
        assert_eq!(e.as_str(), "0 0 8 15 * *");
    }

    #[test]
    fn builder_specific_month() {
        let e = Cron::builder()
            .in_month(12)
            .on_day_of_month(25)
            .at_hour(0)
            .at_minute(0)
            .build();
        assert_eq!(e.as_str(), "0 0 0 25 12 *");
    }

    // ── Parse / validate ──────────────────────────────────────────────────────

    #[test]
    fn parse_valid() {
        assert!(CronExpr::parse("0 0 2 * * *").is_ok());
    }

    #[test]
    fn parse_invalid() {
        assert!(CronExpr::parse("not a cron").is_err());
        assert!(CronExpr::parse("* * *").is_err());
    }

    // ── All presets are valid cron strings ────────────────────────────────────

    #[test]
    fn presets_all_valid() {
        let exprs = [
            Cron::every_minute(),
            Cron::every_n_minutes(5),
            Cron::hourly(),
            Cron::hourly_at(15),
            Cron::every_n_hours(3),
            Cron::daily_at(0, 0),
            Cron::daily_at(23, 59),
            Cron::weekly_on(Weekday::Saturday, 12, 0),
            Cron::monthly_on(1, 0, 0),
            Cron::monthly_on(31, 23, 59),
        ];
        for e in &exprs {
            assert!(
                Schedule::from_str(e.as_str()).is_ok(),
                "Invalid: {}",
                e.as_str()
            );
        }
    }

    // ── describe_cron ─────────────────────────────────────────────────────────

    #[test]
    fn describe_every_minute() {
        assert_eq!(describe_cron("0 * * * * *"), "every minute");
    }

    #[test]
    fn describe_every_n_minutes() {
        assert_eq!(describe_cron("0 */15 * * * *"), "every 15 minutes");
    }

    #[test]
    fn describe_hourly() {
        assert_eq!(describe_cron("0 0 * * * *"), "every hour");
    }

    #[test]
    fn describe_every_n_hours() {
        assert_eq!(describe_cron("0 0 */6 * * *"), "every 6 hours");
    }

    #[test]
    fn describe_daily() {
        assert_eq!(describe_cron("0 0 2 * * *"), "daily at 02:00");
        assert_eq!(describe_cron("0 30 14 * * *"), "daily at 14:30");
    }

    #[test]
    fn describe_weekly() {
        assert_eq!(describe_cron("0 0 0 * * 0"), "every Sunday at 00:00");
        assert_eq!(describe_cron("0 15 9 * * 1"), "every Monday at 09:15");
    }

    #[test]
    fn describe_multiple_weekdays() {
        assert_eq!(
            describe_cron("0 30 9 * * 1,5"),
            "on Monday, Friday at 09:30"
        );
    }

    #[test]
    fn describe_monthly() {
        assert_eq!(
            describe_cron("0 0 3 1 * *"),
            "on day 1 of every month at 03:00"
        );
    }
}
