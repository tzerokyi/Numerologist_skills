//! 紫微斗数 (Ziwei Doushu) native natal chart and fortune layers.
//!
//! The convention is ziwei-sanhe-v1 from ziwei-doushu/references/calculation.md.
//! This module deliberately treats the traditional chart as a deterministic
//! mapping, not an empirical claim about a person's life or future.

use std::collections::BTreeMap;

use lunar_rs::Solar;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Result;
use crate::calendar::{BRANCHES, DateParts, STEMS, TimeInput, as_solar};
use crate::invalid;

const PALACE_NAMES: [&str; 12] = [
    "命宫", "兄弟", "夫妻", "子女", "财帛", "疾厄",
    "迁移", "交友", "官禄", "田宅", "福德", "父母",
];
const MAJOR_A: [(&str, i32); 6] = [
    ("紫微", 0), ("天机", -1), ("太阳", -3),
    ("武曲", -4), ("天同", -5), ("廉贞", -8),
];
const MAJOR_B: [(&str, i32); 8] = [
    ("天府", 0), ("太阴", 1), ("贪狼", 2), ("巨门", 3),
    ("天相", 4), ("天梁", 5), ("七杀", 6), ("破军", 10),
];
const MAJOR_NAMES: [&str; 14] = [
    "紫微", "天机", "太阳", "武曲", "天同", "廉贞", "天府",
    "太阴", "贪狼", "巨门", "天相", "天梁", "七杀", "破军",
];
const MINOR_NAMES: [&str; 14] = [
    "文昌", "文曲", "左辅", "右弼", "天魁", "天钺", "禄存",
    "擎羊", "陀罗", "火星", "铃星", "地空", "地劫", "天马",
];
const ADJECTIVE_NAMES: [&str; 6] = ["天刑", "天姚", "红鸾", "天喜", "天哭", "天虚"];
const SIHUA_TYPES: [&str; 4] = ["化禄", "化权", "化科", "化忌"];
const SIHUA: [[&str; 4]; 10] = [
    ["廉贞", "破军", "武曲", "太阳"],
    ["天机", "天梁", "紫微", "太阴"],
    ["天同", "天机", "文昌", "廉贞"],
    ["太阴", "天同", "天机", "巨门"],
    ["贪狼", "太阴", "右弼", "天机"],
    ["武曲", "贪狼", "天梁", "文曲"],
    ["太阳", "武曲", "太阴", "天同"],
    ["巨门", "太阳", "文曲", "文昌"],
    ["天梁", "紫微", "左辅", "武曲"],
    ["破军", "巨门", "太阴", "贪狼"],
];
const BRIGHTNESS: [&str; 14] = [
    "平 庙 旺 旺 得地 旺 庙 庙 旺 旺 得地 旺",
    "庙 落陷 得地 旺 利 平 庙 落陷 得地 旺 利 平",
    "落陷 不得地 旺 庙 旺 旺 旺 得地 得地 平 不得地 落陷",
    "旺 庙 得地 利 庙 平 旺 庙 得地 利 庙 平",
    "旺 不得地 利 平 平 庙 落陷 不得地 旺 平 平 庙",
    "平 利 庙 平 利 落陷 平 利 庙 平 利 落陷",
    "庙 庙 庙 得地 庙 得地 旺 庙 得地 旺 庙 得地",
    "庙 庙 旺 落陷 落陷 落陷 不得地 不得地 利 旺 旺 庙",
    "旺 庙 平 利 庙 落陷 旺 庙 平 利 庙 落陷",
    "旺 不得地 庙 庙 落陷 旺 旺 不得地 庙 庙 落陷 旺",
    "庙 庙 庙 落陷 得地 得地 庙 得地 庙 落陷 得地 得地",
    "庙 旺 庙 庙 庙 落陷 庙 旺 落陷 得地 庙 落陷",
    "旺 庙 庙 旺 庙 平 旺 庙 庙 旺 庙 平",
    "庙 旺 得地 落陷 旺 平 庙 旺 得地 落陷 旺 平",
];
const MING_ZHU: [&str; 12] = [
    "贪狼", "巨门", "禄存", "文曲", "廉贞", "武曲",
    "破军", "武曲", "廉贞", "文曲", "禄存", "巨门",
];
const SHEN_ZHU: [&str; 12] = [
    "火星", "天相", "天梁", "天同", "文昌", "天机",
    "火星", "天相", "天梁", "天同", "文昌", "天机",
];

fn default_calendar() -> String { "solar".into() }
fn default_ruleset() -> String { "ziwei-sanhe-v1".into() }
fn default_leap_rule() -> String { "split_15".into() }
fn default_late_rule() -> String { "next_day".into() }

/// Explicit birth time is mandatory; unlike Qimen, missing hour is an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthClock {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    #[serde(default)]
    pub minute: i32,
    #[serde(default)]
    pub is_leap_month: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BirthInput {
    Text(String),
    Parts(BirthClock),
}

impl BirthInput {
    fn to_time_input(&self) -> TimeInput {
        match self {
            Self::Text(value) => TimeInput::Text(value.clone()),
            Self::Parts(value) => TimeInput::Parts(DateParts {
                year: value.year,
                month: value.month,
                day: value.day,
                hour: value.hour,
                minute: value.minute,
                second: 0,
                is_leap_month: value.is_leap_month,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiweiRequest {
    #[serde(default = "default_calendar")]
    pub calendar_type: String,
    pub birth_time: BirthInput,
    pub gender: String,
    #[serde(default)]
    pub is_leap_month: bool,
    #[serde(default = "default_leap_rule")]
    pub leap_month_rule: String,
    #[serde(default = "default_late_rule")]
    pub late_zi_rule: String,
    #[serde(default = "default_ruleset")]
    pub ruleset: String,
    #[serde(default)]
    pub query_year: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PalaceRef {
    pub branch: String,
    pub ganzhi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShenPalace {
    pub branch: String,
    pub palace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WuxingJu {
    pub name: String,
    pub number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SihuaItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub star: String,
    pub branch: String,
    pub palace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MajorStar {
    pub name: String,
    pub brightness: String,
    pub sihua: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinorStar {
    pub name: String,
    pub sihua: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgeSpan {
    pub start_age: usize,
    pub end_age: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZiweiPalace {
    pub name: String,
    pub branch: String,
    pub stem: String,
    pub ganzhi: String,
    pub is_shen_gong: bool,
    pub major_stars: Vec<MajorStar>,
    pub minor_stars: Vec<MinorStar>,
    pub adjective_stars: Vec<String>,
    pub decadal: AgeSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelfCheck {
    pub major_star_count: usize,
    pub ziwei_branch: String,
    pub ziwei_partner: Option<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiweiChart {
    pub ming_gong: PalaceRef,
    pub shen_gong: ShenPalace,
    pub wuxing_ju: WuxingJu,
    pub ming_zhu: String,
    pub shen_zhu: String,
    pub decadal_direction: String,
    pub year_sihua: Vec<SihuaItem>,
    pub star_index: BTreeMap<String, String>,
    pub palaces: Vec<ZiweiPalace>,
    pub self_check: SelfCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YearlyPalace {
    pub name: String,
    pub branch: String,
    pub natal_palace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Yearly {
    pub ganzhi: String,
    pub ming_branch: String,
    pub ming_on_natal_palace: String,
    pub palaces: Vec<YearlyPalace>,
    pub sihua: Vec<SihuaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Decadal {
    pub branch: String,
    pub ganzhi: String,
    pub natal_palace: String,
    pub start_age: usize,
    pub end_age: usize,
    pub sihua: Vec<SihuaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Horoscope {
    pub query_lunar_year: i32,
    pub nominal_age: i32,
    pub yearly: Yearly,
    pub decadal: Option<Decadal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiweiOutput {
    pub normalized_input: Value,
    pub calendar: Value,
    pub ruleset: Value,
    pub chart: ZiweiChart,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horoscope: Option<Horoscope>,
}

fn wrap(index: i32) -> usize {
    index.rem_euclid(12) as usize
}

fn palace_name(ming: usize, branch: usize) -> &'static str {
    PALACE_NAMES[wrap(ming as i32 - branch as i32)]
}

fn palace_stem(year_stem_index: usize, branch: usize) -> &'static str {
    let yin_stem_index = [2, 4, 6, 8, 0, 2, 4, 6, 8, 0][year_stem_index];
    STEMS[(yin_stem_index + wrap(branch as i32 - 2)) % 10]
}

fn wuxing_ju(stem_index: usize, branch_index: usize) -> WuxingJu {
    let stem_number = [1, 1, 2, 2, 3, 3, 4, 4, 5, 5][stem_index];
    let branch_number = [1, 1, 2, 2, 3, 3, 1, 1, 2, 2, 3, 3][branch_index];
    let modulo = (stem_number + branch_number - 1) % 5;
    let (name, number) = [
        ("木三局", 3), ("金四局", 4), ("水二局", 2),
        ("火六局", 6), ("土五局", 5),
    ][modulo];
    WuxingJu { name: name.into(), number }
}

/// From the original verse: quotient starts at 寅; odd shortfall backward,
/// even shortfall forward. Test all 30 days for all five 局.
pub fn ziwei_position(ju_number: usize, lunar_day: usize) -> Result<usize> {
    if ![2, 3, 4, 5, 6].contains(&ju_number) || !(1..=30).contains(&lunar_day) {
        return Err(invalid("invalid WuXing Ju or lunar day"));
    }
    let quotient = lunar_day.div_ceil(ju_number);
    let shortfall = quotient * ju_number - lunar_day;
    let base = 2 + quotient as i32 - 1;
    Ok(wrap(if shortfall % 2 == 1 {
        base - shortfall as i32
    } else {
        base + shortfall as i32
    }))
}

fn major_stars(ziwei: usize) -> BTreeMap<String, usize> {
    let mut stars = BTreeMap::new();
    for (name, offset) in MAJOR_A {
        stars.insert(name.into(), wrap(ziwei as i32 + offset));
    }
    let tianfu = wrap(4 - ziwei as i32);
    for (name, offset) in MAJOR_B {
        stars.insert(name.into(), wrap(tianfu as i32 + offset));
    }
    stars
}

fn branch_index(branch: &str) -> Result<usize> {
    BRANCHES.iter().position(|candidate| *candidate == branch).ok_or_else(|| {
        invalid(format!("invalid earthly branch: {branch}"))
    })
}

fn minor_stars(stem: usize, year_branch: usize, month: usize, hour: usize) -> BTreeMap<String, usize> {
    let lucun = [2, 3, 5, 6, 5, 6, 8, 9, 11, 0][stem];
    let (kui, yue) = [
        (1, 7), (0, 8), (11, 9), (11, 9), (1, 7),
        (0, 8), (1, 7), (6, 2), (3, 5), (3, 5),
    ][stem];
    let (huo, ling, tianma) = match year_branch {
        2 | 6 | 10 => (1, 3, 8),  // 寅午戌
        8 | 0 | 4 => (2, 10, 2),  // 申子辰
        5 | 9 | 1 => (3, 10, 11), // 巳酉丑
        _ => (9, 10, 5),         // 亥卯未
    };
    let m = month as i32 - 1;
    let h = hour as i32;
    let positions = [
        wrap(10 - h), wrap(4 + h), wrap(4 + m), wrap(10 - m),
        kui, yue, lucun, wrap(lucun as i32 + 1), wrap(lucun as i32 - 1),
        wrap(huo + h), wrap(ling + h), wrap(11 - h), wrap(11 + h), tianma,
    ];
    MINOR_NAMES.iter().zip(positions).map(|(name, position)| ((*name).into(), position)).collect()
}

fn adjective_stars(year_branch: usize, month: usize) -> BTreeMap<String, usize> {
    let year_branch = year_branch as i32;
    let m = month as i32 - 1;
    let red = wrap(3 - year_branch);
    let positions = [
        wrap(9 + m), wrap(1 + m), red, wrap(red as i32 + 6),
        wrap(6 - year_branch), wrap(6 + year_branch),
    ];
    ADJECTIVE_NAMES.iter().zip(positions).map(|(name, position)| ((*name).into(), position)).collect()
}

fn sihua_for_stem(stem_index: usize, stars: &BTreeMap<String, usize>, ming: usize) -> Result<Vec<SihuaItem>> {
    SIHUA_TYPES.iter().zip(SIHUA[stem_index]).map(|(kind, star)| {
        let branch_index = stars.get(star).ok_or_else(|| invalid(format!("Sihua star not on chart: {star}")))?;
        Ok(SihuaItem {
            kind: (*kind).into(),
            star: star.into(),
            branch: BRANCHES[*branch_index].into(),
            palace: palace_name(ming, *branch_index).into(),
        })
    }).collect()
}

fn self_check(major: &BTreeMap<String, usize>, star_index: &BTreeMap<String, usize>) -> Result<SelfCheck> {
    if major.len() != 14 || MAJOR_NAMES.iter().any(|name| !major.contains_key(*name)) {
        return Err(invalid("incomplete set of fourteen main stars"));
    }
    let ziwei = major["紫微"];
    let allowed = [
        ("天府", [2, 8]), ("贪狼", [3, 9]), ("天相", [4, 10]),
        ("七杀", [5, 11]), ("破军", [1, 7]),
    ];
    let partner = allowed.iter().find_map(|(star, branches)| {
        (major[*star] == ziwei).then_some((*star, *branches))
    });
    if let Some((star, pair)) = partner {
        if !pair.contains(&ziwei) {
            return Err(invalid(format!("invalid Ziwei-{star} pair at {}", BRANCHES[ziwei])));
        }
    }
    if SIHUA.iter().flatten().any(|star| !star_index.contains_key(*star)) {
        return Err(invalid("incomplete Sihua star indexing"));
    }
    Ok(SelfCheck {
        major_star_count: 14,
        ziwei_branch: BRANCHES[ziwei].into(),
        ziwei_partner: partner.map(|(name, _)| name.into()),
        passed: true,
    })
}

fn brightness(star: &str, branch: usize) -> Result<String> {
    let index = MAJOR_NAMES.iter().position(|name| *name == star).ok_or_else(|| invalid("invalid major star"))?;
    BRIGHTNESS[index]
        .split_whitespace()
        .nth(branch)
        .map(str::to_owned)
        .ok_or_else(|| invalid("missing brightness reference"))
}

fn make_horoscope(
    query_year: i32,
    lunar_year: i32,
    ming: usize,
    stars: &BTreeMap<String, usize>,
    stems: &[usize; 12],
    decadal: &[AgeSpan; 12],
) -> Result<Horoscope> {
    let nominal_age = query_year - lunar_year + 1;
    let year_stem = (query_year - 4).rem_euclid(10) as usize;
    let yearly_branch = (query_year - 4).rem_euclid(12) as usize;
    let yearly = Yearly {
        ganzhi: format!("{}{}", STEMS[year_stem], BRANCHES[yearly_branch]),
        ming_branch: BRANCHES[yearly_branch].into(),
        ming_on_natal_palace: palace_name(ming, yearly_branch).into(),
        palaces: (0..12).map(|index| {
            let branch = wrap(yearly_branch as i32 - index);
            YearlyPalace {
                name: PALACE_NAMES[index].into(),
                branch: BRANCHES[branch].into(),
                natal_palace: palace_name(ming, branch).into(),
            }
        }).collect(),
        sihua: sihua_for_stem(year_stem, stars, ming)?,
    };
    let decadal = (0..12).find_map(|index| {
        let age = &decadal[index];
        (nominal_age >= age.start_age as i32 && nominal_age <= age.end_age as i32)
            .then_some((index, age))
    }).map(|(index, age)| {
        let stem = stems[index];
        Ok(Decadal {
            branch: BRANCHES[index].into(),
            ganzhi: format!("{}{}", STEMS[stem], BRANCHES[index]),
            natal_palace: palace_name(ming, index).into(),
            start_age: age.start_age,
            end_age: age.end_age,
            sihua: sihua_for_stem(stem, stars, ming)?,
        })
    }).transpose()?;
    Ok(Horoscope { query_lunar_year: query_year, nominal_age, yearly, decadal })
}

/// Build a natal chart, optional 10-year cycle and query-year structures.
/// Gregorian / lunar conversions use the pinned native astronomical engine.
pub fn calculate(request: &ZiweiRequest) -> Result<ZiweiOutput> {
    let gender = match request.gender.trim().to_lowercase().as_str() {
        "男" | "male" | "m" => "男",
        "女" | "female" | "f" => "女",
        _ => return Err(invalid("gender is required: 男/女 (or male/female)")),
    };
    if !["current", "next", "split_15"].contains(&request.leap_month_rule.as_str()) {
        return Err(invalid("leap_month_rule must be current, next or split_15"));
    }
    if !["next_day", "same_day"].contains(&request.late_zi_rule.as_str()) {
        return Err(invalid("late_zi_rule must be next_day or same_day"));
    }
    let input = request.birth_time.to_time_input();
    let parts = input.parts(true)?;
    if !(0..=23).contains(&parts.hour) || !(0..=59).contains(&parts.minute) {
        return Err(invalid("birth time must use 00:00..23:59"));
    }
    let solar = as_solar(&request.calendar_type, &input, request.is_leap_month, true)?;
    let day_rollover = parts.hour == 23 && request.late_zi_rule == "next_day";
    let chart_solar: Solar = if day_rollover { solar.next_day(1) } else { solar };
    let lunar = chart_solar.lunar();
    let lunar_month = lunar.month().unsigned_abs() as usize;
    let lunar_day = lunar.day() as usize;
    let is_leap = lunar.month() < 0;
    let chart_month = if is_leap
        && (request.leap_month_rule == "next"
            || (request.leap_month_rule == "split_15" && lunar_day > 15))
    {
        lunar_month % 12 + 1
    } else {
        lunar_month
    };
    let year_stem = lunar.year_gan();
    let year_branch = lunar.year_zhi();
    let stem_index = STEMS.iter().position(|candidate| *candidate == year_stem)
        .ok_or_else(|| invalid("invalid lunar year stem"))?;
    let year_branch_index = branch_index(year_branch)?;
    let hour_index = ((parts.hour + 1) / 2) as usize % 12;
    let month_palace = wrap(2 + chart_month as i32 - 1);
    let ming = wrap(month_palace as i32 - hour_index as i32);
    let shen = wrap(month_palace as i32 + hour_index as i32);
    let stem_indices: [usize; 12] = std::array::from_fn(|index| {
        let stem = palace_stem(stem_index, index);
        STEMS.iter().position(|candidate| *candidate == stem).expect("valid stem")
    });
    let ju = wuxing_ju(stem_indices[ming], ming);
    let ziwei = ziwei_position(ju.number, lunar_day)?;
    let major = major_stars(ziwei);
    let minor = minor_stars(stem_index, year_branch_index, chart_month, hour_index);
    let adjective = adjective_stars(year_branch_index, chart_month);
    let mut star_positions = major.clone();
    star_positions.extend(minor.iter().map(|(name, idx)| (name.clone(), *idx)));
    star_positions.extend(adjective.iter().map(|(name, idx)| (name.clone(), *idx)));
    let check = self_check(&major, &star_positions)?;
    let year_sihua = sihua_for_stem(stem_index, &star_positions, ming)?;
    let sihua_by_star: BTreeMap<String, String> = year_sihua.iter()
        .map(|item| (item.star.clone(), item.kind.clone())).collect();
    let is_yang_year = stem_index % 2 == 0;
    let direction: i32 = if is_yang_year == (gender == "男") { 1 } else { -1 };
    let mut decadal: [AgeSpan; 12] = std::array::from_fn(|_| AgeSpan { start_age: 0, end_age: 0 });
    for step in 0..12 {
        let branch = wrap(ming as i32 + direction * step);
        let start_age = ju.number + 10 * step;
        decadal[branch] = AgeSpan { start_age, end_age: start_age + 9 };
    }

    let mut palaces = Vec::with_capacity(12);
    for (index, name) in PALACE_NAMES.iter().enumerate() {
        let branch = wrap(ming as i32 - index as i32);
        let stem = STEMS[stem_indices[branch]];
        let major_stars = MAJOR_NAMES.iter()
            .filter(|star| major.get(**star) == Some(&branch))
            .map(|star| Ok(MajorStar {
                name: (*star).into(),
                brightness: brightness(star, branch)?,
                sihua: sihua_by_star.get(*star).cloned(),
            }))
            .collect::<Result<Vec<_>>>()?;
        let minor_stars = MINOR_NAMES.iter()
            .filter(|star| minor.get(**star) == Some(&branch))
            .map(|star| MinorStar {
                name: (*star).into(),
                sihua: sihua_by_star.get(*star).cloned(),
            })
            .collect();
        let adjective_stars = ADJECTIVE_NAMES.iter()
            .filter(|star| adjective.get(**star) == Some(&branch))
            .map(|star| (*star).into())
            .collect();
        palaces.push(ZiweiPalace {
            name: (*name).into(),
            branch: BRANCHES[branch].into(),
            stem: stem.into(),
            ganzhi: format!("{stem}{}", BRANCHES[branch]),
            is_shen_gong: branch == shen,
            major_stars,
            minor_stars,
            adjective_stars,
            decadal: decadal[branch].clone(),
        });
    }
    let star_index = star_positions.iter()
        .map(|(name, index)| (name.clone(), BRANCHES[*index].into())).collect();
    let horoscope = request.query_year.map(|year| {
        make_horoscope(year, lunar.year(), ming, &star_positions, &stem_indices, &decadal)
    }).transpose()?;
    let mut warnings = Vec::new();
    let minutes = parts.hour * 60 + parts.minute;
    let boundary = (minutes - 60).rem_euclid(120);
    if boundary.min(120 - boundary) <= 15 {
        warnings.push("出生时间距时辰交界不足 15 分钟，时辰有误差时盘面会整体改变，建议按相邻两个时辰各排一盘对照。".into());
    }
    if day_rollover {
        warnings.push("出生于 23 点后，已按次日子时排盘（农历日期取次日）；若用户既定口径不换日，请改用 late_zi_rule=same_day 重排。".into());
    }
    if is_leap {
        warnings.push(format!(
            "出生于农历闰{lunar_month}月{lunar_day}日，闰月按 {} 定为{chart_month}月。",
            request.leap_month_rule,
        ));
    }
    let lunar_text = format!("{}年{}月{}", lunar.year_in_gan_zhi(), lunar.month_in_chinese(), lunar.day_in_chinese());
    let chart = ZiweiChart {
        ming_gong: PalaceRef {
            branch: BRANCHES[ming].into(),
            ganzhi: format!("{}{}", STEMS[stem_indices[ming]], BRANCHES[ming]),
        },
        shen_gong: ShenPalace {
            branch: BRANCHES[shen].into(),
            palace: palace_name(ming, shen).into(),
        },
        wuxing_ju: ju,
        ming_zhu: MING_ZHU[ming].into(),
        shen_zhu: SHEN_ZHU[year_branch_index].into(),
        decadal_direction: if direction == 1 { "顺行" } else { "逆行" }.into(),
        year_sihua,
        star_index,
        palaces,
        self_check: check,
    };
    Ok(ZiweiOutput {
        normalized_input: json!({
            "calendar_type": request.calendar_type,
            "original_birth_time": request.birth_time,
            "civil_solar_datetime": format!("{:04}-{:02}-{:02} {:02}:{:02}",
                solar.year(), solar.month(), solar.day(), parts.hour, parts.minute),
            "gender": gender,
            "leap_month_rule": request.leap_month_rule,
            "leap_month_rule_source": "explicit",
            "late_zi_rule": request.late_zi_rule
        }),
        calendar: json!({
            "chart_solar_date": chart_solar.to_ymd(),
            "day_rollover_applied": day_rollover,
            "lunar_year": lunar.year(),
            "lunar_month": lunar_month,
            "lunar_day": lunar_day,
            "is_leap_month": is_leap,
            "chart_month": chart_month,
            "lunar_text": lunar_text,
            "year_ganzhi": lunar.year_in_gan_zhi(),
            "year_yinyang": if is_yang_year { "阳" } else { "阴" },
            "hour_branch": format!("{}时", BRANCHES[hour_index])
        }),
        ruleset: json!({
            "id": request.ruleset,
            "source": "references/calculation.md、references/sihua.md、references/stars.md",
            "year_rule": "以农历正月初一换年，年干支取农历年。",
            "late_zi_rule": if request.late_zi_rule == "next_day" {
                "23:00 起视为次日子时，农历日期按次日计算（calculation.md 默认口径）"
            } else {
                "23:00 起取子时，但农历日期仍按当天（沿用用户既定口径时使用）"
            },
            "leap_month_rule": {
                "id": request.leap_month_rule,
                "source": "explicit",
                "description": match request.leap_month_rule.as_str() {
                    "current" => "闰月整月按本月",
                    "next" => "闰月整月按下月",
                    _ => "闰月十五日及以前按本月，十六日起按下月"
                },
                "applied": is_leap
            },
            "age_rule": "大限年龄为虚岁。"
        }),
        chart,
        warnings,
        horoscope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(time: &str, gender: &str) -> ZiweiRequest {
        ZiweiRequest {
            calendar_type: "solar".into(),
            birth_time: BirthInput::Text(time.into()),
            gender: gender.into(),
            is_leap_month: false,
            leap_month_rule: "split_15".into(),
            late_zi_rule: "next_day".into(),
            ruleset: "ziwei-sanhe-v1".into(),
            query_year: None,
        }
    }

    #[test]
    fn canonical_hand_chart() {
        let output = calculate(&request("2000-08-16 14:30", "女")).unwrap();
        assert_eq!(output.calendar["lunar_text"], "庚辰年七月十七");
        assert_eq!(output.chart.ming_gong.ganzhi, "己丑");
        assert_eq!(output.chart.shen_gong.branch, "卯");
        assert_eq!(output.chart.shen_gong.palace, "福德");
        assert_eq!(output.chart.wuxing_ju, WuxingJu { name: "火六局".into(), number: 6 });
        assert_eq!(output.chart.decadal_direction, "逆行");
        assert_eq!(output.chart.star_index["紫微"], "卯");
        assert_eq!(output.chart.star_index["天府"], "丑");
        assert_eq!(output.chart.star_index["七杀"], "未");
        assert_eq!(output.chart.year_sihua[0].star, "太阳");
        assert_eq!(output.chart.palaces[0].decadal, AgeSpan { start_age: 6, end_age: 15 });
    }

    #[test]
    fn late_zi_crossover_and_leap_month() {
        let late = calculate(&request("2000-08-16 23:30", "男")).unwrap();
        let early = calculate(&request("2000-08-17 00:30", "男")).unwrap();
        assert_eq!(late.chart.star_index, early.chart.star_index);
        assert_eq!(late.calendar["lunar_day"], 18);
        let mut same = request("2000-08-16 23:30", "男");
        same.late_zi_rule = "same_day".into();
        assert_eq!(calculate(&same).unwrap().calendar["lunar_day"], 17);
        assert_eq!(calculate(&request("2023-04-05 10:00", "男")).unwrap().calendar["chart_month"], 2);
        assert_eq!(calculate(&request("2023-04-06 10:00", "男")).unwrap().calendar["chart_month"], 3);
    }

    #[test]
    fn old_and_new_lunar_year_boundary() {
        let old = calculate(&request("2024-02-05 10:00", "男")).unwrap();
        let new = calculate(&request("2024-02-10 10:00", "男")).unwrap();
        assert_eq!(old.calendar["year_ganzhi"], "癸卯");
        assert_eq!(new.calendar["year_ganzhi"], "甲辰");
    }

    #[test]
    fn horoscope_year_and_decade() {
        let mut req = request("2000-08-16 14:30", "女");
        req.query_year = Some(2026);
        let report = calculate(&req).unwrap();
        let horoscope = report.horoscope.unwrap();
        assert_eq!(horoscope.nominal_age, 27);
        assert_eq!(horoscope.yearly.ganzhi, "丙午");
        assert_eq!(horoscope.yearly.ming_on_natal_palace, "交友");
        assert_eq!(horoscope.decadal.unwrap().ganzhi, "丁亥");
    }

    #[test]
    fn table_and_verse_properties() {
        for ju in [2, 3, 4, 5, 6] {
            for day in 1..=30 {
                assert!(ziwei_position(ju, day).unwrap() < 12);
            }
        }
        for ziwei in 0..12 {
            assert_eq!(major_stars(ziwei).len(), 14);
        }
        assert!(ziwei_position(3, 31).is_err());
    }

    #[test]
    fn requires_gender_and_hour() {
        let mut req = request("2000-08-16 14:30", "?");
        assert!(calculate(&req).is_err());
        req.gender = "男".into();
        req.birth_time = BirthInput::Text("2000-08-16".into());
        assert!(calculate(&req).is_err());
        let missing_hour = r#"{"calendar_type":"solar","birth_time":{"year":2000,"month":8,"day":16},"gender":"女"}"#;
        assert!(serde_json::from_str::<ZiweiRequest>(missing_hour).is_err());
    }
}
