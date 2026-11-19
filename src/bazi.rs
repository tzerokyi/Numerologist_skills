//! Four pillars (BaZi): precise Ganzhi, Ten Gods and hidden heavenly stems.
//!
//! Rule provenance: bazi/SKILL.md and bazi/references/tiangan-dizhi.md.
//! No deterministic strength, fate, favorable element or personality conclusion
//! is invented from a partial traditional rule set.

use lunar_rs::Solar;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::calendar::{BRANCHES, STEMS, TimeInput, as_solar, branch_index, stem_index};
use crate::invalid;

fn default_calendar() -> String {
    "solar".into()
}

fn default_late_zi_rule() -> String {
    "same_day".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaziRequest {
    #[serde(default = "default_calendar")]
    pub calendar_type: String,
    pub birth_time: TimeInput,
    #[serde(default)]
    pub is_leap_month: bool,
    /// BaZi default: 夜子时, 23:00 stays on the same civil day.
    /// Alternative next_day explicitly applies 23:00 rollover.
    #[serde(default = "default_late_zi_rule")]
    pub late_zi_rule: String,
    /// Optional male/female interpretation only for the 10-year luck direction.
    #[serde(default)]
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FourPillars {
    pub year: String,
    pub month: String,
    pub day: String,
    pub hour: String,
    pub solar_datetime: String,
    pub calculation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StemInfo {
    pub name: String,
    pub yin_yang: String,
    pub element: String,
    /// None for the day master itself; other stems are compared to the day master.
    pub ten_god: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PillarInfo {
    pub name: String,
    pub ganzhi: String,
    pub stem: StemInfo,
    pub branch: String,
    pub branch_element: String,
    pub hidden_stems: Vec<StemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaziOutput {
    #[serde(flatten)]
    pub four_pillars: FourPillars,
    pub day_master: StemInfo,
    pub pillars: [PillarInfo; 4],
    pub luck_direction: Option<String>,
    pub late_zi_rule: String,
    pub warnings: Vec<String>,
}

const HIDDEN_STEMS: [&[usize]; 12] = [
    &[9],          // 子 癸
    &[5, 7, 9],    // 丑 己辛癸
    &[0, 2, 4],    // 寅 甲丙戊
    &[1],          // 卯 乙
    &[4, 1, 9],    // 辰 戊乙癸
    &[2, 4, 6],    // 巳 丙戊庚
    &[3, 5],       // 午 丁己
    &[5, 3, 1],    // 未 己丁乙
    &[6, 8, 4],    // 申 庚壬戊
    &[7],          // 酉 辛
    &[4, 7, 3],    // 戌 戊辛丁
    &[8, 0],       // 亥 壬甲
];

const BRANCH_ELEMENTS: [&str; 12] = [
    "水", "土", "木", "木", "土", "火",
    "火", "土", "金", "金", "土", "水",
];
const STEM_ELEMENTS: [&str; 5] = ["木", "火", "土", "金", "水"];

/// The deterministic Ten Gods mapping supplied by upstream reference tables.
/// Returns the traditional structural label, not a prediction.
pub fn ten_god(day_stem: &str, other_stem: &str) -> Result<&'static str> {
    let day = stem_index(day_stem)?;
    let other = stem_index(other_stem)?;
    let self_element = day / 2;
    let other_element = other / 2;
    let same_polarity = day % 2 == other % 2;
    let label = if self_element == other_element {
        if same_polarity { "比肩" } else { "劫财" }
    } else if (other_element + 1) % 5 == self_element {
        if same_polarity { "偏印" } else { "正印" }
    } else if (self_element + 1) % 5 == other_element {
        if same_polarity { "食神" } else { "伤官" }
    } else if (self_element + 2) % 5 == other_element {
        if same_polarity { "偏财" } else { "正财" }
    } else {
        if same_polarity { "七杀" } else { "正官" }
    };
    Ok(label)
}

fn stem_info(index: usize, day: usize, is_day_master: bool) -> StemInfo {
    StemInfo {
        name: STEMS[index].into(),
        yin_yang: if index % 2 == 0 { "阳" } else { "阴" }.into(),
        element: STEM_ELEMENTS[index / 2].into(),
        ten_god: if is_day_master {
            None
        } else {
            Some(ten_god(STEMS[day], STEMS[index]).expect("validated stem indices").into())
        },
    }
}

fn pillar_info(name: &str, ganzhi: &str, day_stem: usize, is_day: bool) -> Result<PillarInfo> {
    let stem = ganzhi.chars().next().ok_or_else(|| invalid("empty Ganzhi"))?.to_string();
    let branch = ganzhi.chars().nth(1).ok_or_else(|| invalid("incomplete Ganzhi"))?.to_string();
    let stem_index = stem_index(&stem)?;
    let branch_index = branch_index(&branch)?;
    Ok(PillarInfo {
        name: name.into(),
        ganzhi: ganzhi.into(),
        stem: stem_info(stem_index, day_stem, is_day),
        branch,
        branch_element: BRANCH_ELEMENTS[branch_index].into(),
        hidden_stems: HIDDEN_STEMS[branch_index].iter()
            .map(|&index| stem_info(index, day_stem, false))
            .collect(),
    })
}

/// Calculate the deterministic portions of the upstream BaZi reference.
/// All four pillars use the given *local civil time*, without auto-TZ guessing.
pub fn calculate(request: &BaziRequest) -> Result<BaziOutput> {
    if !["same_day", "next_day"].contains(&request.late_zi_rule.as_str()) {
        return Err(invalid("late_zi_rule must be same_day or next_day"));
    }
    let time = request.birth_time.parts(true)?;
    let solar: Solar = as_solar(
        &request.calendar_type,
        &request.birth_time,
        request.is_leap_month,
        true,
    )?;
    let lunar = solar.lunar();
    let day = if request.late_zi_rule == "same_day" {
        lunar.day_in_gan_zhi_exact2()
    } else {
        lunar.day_in_gan_zhi_exact()
    };
    let day_stem = day.chars().next().ok_or_else(|| invalid("invalid day pillar"))?.to_string();
    let day_index = stem_index(&day_stem)?;
    let hour_branch = ((time.hour + 1) / 2).rem_euclid(12) as usize;
    let hour_stem = (day_index % 5 * 2 + hour_branch) % 10;
    let hour = format!("{}{}", STEMS[hour_stem], BRANCHES[hour_branch]);
    let year = lunar.year_in_gan_zhi_exact();
    let month = lunar.month_in_gan_zhi_exact();
    let four_pillars = FourPillars {
        year,
        month,
        day,
        hour,
        solar_datetime: solar.to_ymd_hms(),
        calculation: "JieQi year/month, selected late-Zi day convention, matching hour stem".into(),
    };
    let values = [
        ("年柱", &four_pillars.year, false),
        ("月柱", &four_pillars.month, false),
        ("日柱", &four_pillars.day, true),
        ("时柱", &four_pillars.hour, false),
    ];
    let pillars = values.map(|(name, ganzhi, is_day)| {
        pillar_info(name, ganzhi, day_index, is_day)
    }).into_iter().collect::<Result<Vec<_>>>()?
        .try_into().map_err(|_| invalid("expected four pillars"))?;
    let gender = match request.gender.as_deref().map(|g| g.trim().to_lowercase()) {
        None => None,
        Some(value) if ["男", "male", "m"].contains(&value.as_str()) => Some(true),
        Some(value) if ["女", "female", "f"].contains(&value.as_str()) => Some(false),
        _ => return Err(invalid("gender must be 男/女 or male/female")),
    };
    let luck_direction = gender.map(|male| {
        let year_stem = four_pillars.year.chars().next().unwrap_or('甲').to_string();
        let year_stem_index = stem_index(&year_stem).expect("validated year stem");
        if (year_stem_index % 2 == 0) == male { "顺行" } else { "逆行" }.into()
    });
    let mut warnings = Vec::new();
    if time.hour == 23 {
        warnings.push(format!(
            "23:00 子时按 {} 规则排日柱；使用另一口径时应重排。",
            request.late_zi_rule
        ));
    }
    warnings.push("日主强弱、格局和喜用神需要额外规则，不由此确定性核心臆断。".into());
    Ok(BaziOutput {
        day_master: stem_info(day_index, day_index, true),
        four_pillars,
        pillars,
        luck_direction,
        late_zi_rule: request.late_zi_rule.clone(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(birth_time: &str) -> BaziRequest {
        BaziRequest {
            calendar_type: "solar".into(),
            birth_time: TimeInput::Text(birth_time.into()),
            is_leap_month: false,
            late_zi_rule: "same_day".into(),
            gender: None,
        }
    }

    #[test]
    fn ten_gods_all_five_elements_and_polarities() {
        let cases = [
            ("甲", "甲", "比肩"), ("甲", "乙", "劫财"),
            ("甲", "壬", "偏印"), ("甲", "癸", "正印"),
            ("甲", "丙", "食神"), ("甲", "丁", "伤官"),
            ("甲", "戊", "偏财"), ("甲", "己", "正财"),
            ("甲", "庚", "七杀"), ("甲", "辛", "正官"),
        ];
        for (day, other, expected) in cases {
            assert_eq!(ten_god(day, other).unwrap(), expected);
        }
    }

    #[test]
    fn hidden_stems_match_upstream_reference() {
        assert_eq!(HIDDEN_STEMS[0], &[9]);
        assert_eq!(HIDDEN_STEMS[1], &[5, 7, 9]);
        assert_eq!(HIDDEN_STEMS[11], &[8, 0]);
        let chart = calculate(&request("2026-03-24 10:30")).unwrap();
        assert_eq!(chart.pillars.len(), 4);
        assert_eq!(chart.pillars[2].stem.ten_god, None);
        assert_eq!(chart.day_master.name, chart.pillars[2].stem.name);
    }

    #[test]
    fn default_late_zi_uses_same_civil_day() {
        let late = calculate(&request("2026-03-24 23:30")).unwrap();
        let early = calculate(&request("2026-03-24 22:30")).unwrap();
        assert_eq!(late.four_pillars.day, early.four_pillars.day);
        let mut next_day = request("2026-03-24 23:30");
        next_day.late_zi_rule = "next_day".into();
        let following = calculate(&next_day).unwrap();
        let midnight = calculate(&request("2026-03-25 00:30")).unwrap();
        assert_ne!(late.four_pillars.day, following.four_pillars.day);
        assert_eq!(following.four_pillars.day, midnight.four_pillars.day);
    }

    #[test]
    fn male_female_luck_direction() {
        let mut male = request("2026-03-24 10:30");
        male.gender = Some("男".into());
        let mut female = male.clone();
        female.gender = Some("女".into());
        assert_ne!(calculate(&male).unwrap().luck_direction, calculate(&female).unwrap().luck_direction);
    }
}
