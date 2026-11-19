//! Input parsing and lunar/solar calendar boundaries shared by chart engines.
//!
//! Inputs represent civil clock time as entered. The named timezone is metadata:
//! the calendar engine uses the provided local civil time, not host TZ inference.

use lunar_rs::{Lunar, Solar};
use serde::{Deserialize, Serialize};

use crate::{Result, invalid};

pub const STEMS: [&str; 10] = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];
pub const BRANCHES: [&str; 12] = [
    "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DateParts {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    #[serde(default)]
    pub hour: i32,
    #[serde(default)]
    pub minute: i32,
    #[serde(default)]
    pub second: i32,
    #[serde(default)]
    pub is_leap_month: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TimeInput {
    Text(String),
    Parts(DateParts),
}

impl TimeInput {
    pub fn parts(&self, require_hour: bool) -> Result<DateParts> {
        match self {
            Self::Parts(parts) => Ok(*parts),
            Self::Text(text) => parse_datetime(text, require_hour),
        }
    }
}

pub fn parse_datetime(text: &str, require_hour: bool) -> Result<DateParts> {
    let normalized = text
        .trim()
        .replace('T', " ")
        .replace('/', "-")
        .replace('年', "-")
        .replace('月', "-")
        .replace('日', " ")
        .replace('时', ":")
        .replace('點', ":")
        .replace('点', ":")
        .replace('分', "")
        .replace('秒', "");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let date = tokens.first().ok_or_else(|| invalid("date/time is empty"))?;
    let parse = |field: &str| -> Result<i32> {
        field
            .parse::<i32>()
            .map_err(|_| invalid(format!("invalid numeric date/time field: {field}")))
    };
    let fields: Vec<&str> = date.split('-').filter(|s| !s.is_empty()).collect();
    if fields.len() != 3 {
        return Err(invalid("date format must be YYYY-MM-DD"));
    }
    if require_hour && tokens.len() < 2 {
        return Err(invalid("birth time must include an hour"));
    }
    let clock = tokens
        .get(1)
        .map(|s| s.split(':').filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    if require_hour && clock.is_empty() {
        return Err(invalid("birth time must include an hour"));
    }
    if clock.len() > 3 {
        return Err(invalid("time format must be HH:MM[:SS]"));
    }
    let component = |index: usize| -> Result<i32> {
        clock.get(index).map_or(Ok(0), |value| parse(value))
    };
    Ok(DateParts {
        year: parse(fields[0])?,
        month: parse(fields[1])?,
        day: parse(fields[2])?,
        hour: component(0)?,
        minute: component(1)?,
        second: component(2)?,
        is_leap_month: false,
    })
}

pub fn as_solar(
    calendar_type: &str,
    input: &TimeInput,
    is_leap_month: bool,
    require_hour: bool,
) -> Result<Solar> {
    let parts = input.parts(require_hour)?;
    match calendar_type {
        "solar" => Solar::from_ymd_hms(
            parts.year,
            parts.month,
            parts.day,
            parts.hour,
            parts.minute,
            parts.second,
        )
        .map_err(|error| invalid(format!("invalid solar date/time: {error}"))),
        "lunar" => Lunar::from_ymd_hms(
            parts.year,
            if is_leap_month || parts.is_leap_month {
                -parts.month
            } else {
                parts.month
            },
            parts.day,
            parts.hour,
            parts.minute,
            parts.second,
        )
        .map(|lunar| lunar.solar())
        .map_err(|error| invalid(format!("invalid lunar date/time: {error}"))),
        "now" => Err(invalid(
            "calendar_type=now is not deterministic; supply an explicit local time",
        )),
        _ => Err(invalid(format!("unsupported calendar_type: {calendar_type}"))),
    }
}

pub fn cycle_index(ganzhi: &str) -> Result<usize> {
    (0..60)
        .find(|&i| format!("{}{}", STEMS[i % 10], BRANCHES[i % 12]) == ganzhi)
        .ok_or_else(|| invalid(format!("invalid sexagenary pair: {ganzhi}")))
}

pub fn xun_from_ganzhi(ganzhi: &str) -> Result<(&'static str, usize)> {
    let index = cycle_index(ganzhi)?;
    let block = index / 10;
    const XUN: [&str; 6] = ["甲子", "甲戌", "甲申", "甲午", "甲辰", "甲寅"];
    Ok((XUN[block], index % 10))
}

pub fn branch_index(branch: &str) -> Result<usize> {
    BRANCHES.iter().position(|candidate| *candidate == branch).ok_or_else(|| {
        invalid(format!("unknown earthly branch: {branch}"))
    })
}

pub fn stem_index(stem: &str) -> Result<usize> {
    STEMS
        .iter()
        .position(|candidate| *candidate == stem)
        .ok_or_else(|| invalid(format!("unknown heavenly stem: {stem}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_origin_formats() {
        let a = parse_datetime("2026年03月24日 10时30分", false).unwrap();
        let b = parse_datetime("2026-03-24T10:30", false).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.hour, 10);
    }

    #[test]
    fn validates_cycle_pairs() {
        assert_eq!(cycle_index("甲子").unwrap(), 0);
        assert_eq!(cycle_index("癸亥").unwrap(), 59);
        assert!(cycle_index("甲丑").is_err());
        assert_eq!(xun_from_ganzhi("乙巳").unwrap(), ("甲辰", 1));
    }

    #[test]
    fn solar_to_lunar_round_trip() {
        let solar = as_solar(
            "lunar",
            &TimeInput::Parts(DateParts {
                year: 2023,
                month: 2,
                day: 1,
                hour: 10,
                minute: 0,
                second: 0,
                is_leap_month: true,
            }),
            false,
            true,
        )
        .unwrap();
        assert_eq!(solar.to_ymd(), "2023-03-22");
    }
}
