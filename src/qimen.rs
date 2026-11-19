//! 时家转盘奇门 (rotating Qimen Dunjia).
//! Rules and original fixtures: ../qimen-dunjia/references and scripts/tests.
//! Source model: exact 24 JieQi (including 中气), 5-day yuan, 9-palace Earth plate.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Result;
use crate::calendar::{TimeInput, cycle_index, xun_from_ganzhi, as_solar};
use crate::invalid;

const RING: [usize; 8] = [1, 8, 3, 4, 9, 2, 7, 6];
const STARS: [&str; 8] = ["天蓬", "天任", "天冲", "天辅", "天英", "天芮", "天柱", "天心"];
const DOORS: [&str; 8] = ["休门", "生门", "伤门", "杜门", "景门", "死门", "惊门", "开门"];
const GODS_YANG: [&str; 8] = ["值符", "螣蛇", "太阴", "六合", "白虎", "玄武", "九地", "九天"];
const GODS_YIN: [&str; 8] = ["值符", "九天", "九地", "玄武", "白虎", "六合", "太阴", "螣蛇"];
const GRID_ORDER: [usize; 9] = [4, 9, 2, 3, 5, 7, 8, 1, 6];

fn default_calendar() -> String {
    "solar".into()
}
fn default_ruleset() -> String {
    "mainline-cn-v1".into()
}
fn default_timezone() -> String {
    "Asia/Shanghai".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Location {
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub city: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QimenRequest {
    #[serde(default)]
    pub question_type: String,
    #[serde(default)]
    pub question_goal: String,
    #[serde(default = "default_calendar")]
    pub calendar_type: String,
    pub time_input: TimeInput,
    #[serde(default)]
    pub is_leap_month: bool,
    #[serde(default)]
    pub location: Location,
    #[serde(default = "default_ruleset")]
    pub ruleset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GanPalace {
    pub stem: String,
    pub raw_palace: Option<usize>,
    pub palace: Option<usize>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Yima {
    pub branch: String,
    pub palace: usize,
    pub source_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Zhifu {
    pub star: String,
    pub palace: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Zhishi {
    pub door: String,
    pub palace: usize,
    pub raw_palace: usize,
    pub hours_from_xunshou: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CenterHosting {
    pub hosted_star: String,
    pub center_stem: String,
    pub earth_host_palace: usize,
    pub sky_palace: usize,
    pub follows_star: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pattern {
    pub name: String,
    pub palace: usize,
    pub detail: String,
    pub nature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QimenPalace {
    pub palace: usize,
    pub name: String,
    pub direction: String,
    pub trigram: String,
    pub element: String,
    pub earth_stem: String,
    pub sky_stem: Option<String>,
    pub sky_stems: Vec<String>,
    pub stem_relation: Option<String>,
    pub hosted_sky_stem: Option<String>,
    pub hosted_stem_relation: Option<String>,
    pub star: Option<String>,
    pub hosted_star: Option<String>,
    pub star_element: Option<String>,
    pub star_palace_relation: Option<String>,
    pub door: Option<String>,
    pub door_element: Option<String>,
    pub door_palace_relation: Option<String>,
    pub god: Option<String>,
    pub is_center: bool,
    pub hosts_center: bool,
    pub hosts_center_sky: bool,
    pub hosting_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QimenChart {
    pub dun_type: String,
    pub yuan: String,
    pub ju_number: usize,
    pub xunshou: String,
    pub hidden_yi: String,
    pub kongwang: Vec<String>,
    pub kongwang_palaces: Vec<usize>,
    pub day_kongwang: Vec<String>,
    pub day_kongwang_palaces: Vec<usize>,
    pub time_stem_visible: String,
    pub day_stem: GanPalace,
    pub year_stem: GanPalace,
    pub month_stem: GanPalace,
    pub hour_yima: Yima,
    pub day_yima: Yima,
    pub zhifu: Zhifu,
    pub zhishi: Zhishi,
    pub center_hosting: CenterHosting,
    pub door_index: BTreeMap<String, usize>,
    pub star_index: BTreeMap<String, usize>,
    pub detected_patterns: Vec<Pattern>,
    pub grid_order: [usize; 9],
    pub palaces: Vec<QimenPalace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QimenOutput {
    pub normalized_input: Value,
    pub calendar: Value,
    pub ganzhi: Value,
    pub ruleset: Value,
    pub chart: QimenChart,
    pub warnings: Vec<String>,
}

fn rotate<T: Clone + PartialEq>(items: &[T], start: &T) -> Vec<T> {
    let index = items.iter().position(|item| item == start).expect("internal rotation start");
    items[index..].iter().chain(&items[..index]).cloned().collect()
}

fn ju_for_term(term: &str) -> Result<(bool, [usize; 3])> {
    // 上、中、下元；from original JU_TABLE, all 24 solar terms including 中气.
    let result = match term {
        "冬至" => (true, [1, 7, 4]),
        "小寒" => (true, [2, 8, 5]),
        "大寒" => (true, [3, 9, 6]),
        "立春" => (true, [8, 5, 2]),
        "雨水" => (true, [9, 6, 3]),
        "惊蛰" => (true, [1, 7, 4]),
        "春分" => (true, [3, 9, 6]),
        "清明" => (true, [4, 1, 7]),
        "谷雨" => (true, [5, 2, 8]),
        "立夏" => (true, [4, 1, 7]),
        "小满" => (true, [5, 2, 8]),
        "芒种" => (true, [6, 3, 9]),
        "夏至" => (false, [9, 3, 6]),
        "小暑" => (false, [8, 2, 5]),
        "大暑" => (false, [7, 1, 4]),
        "立秋" => (false, [2, 5, 8]),
        "处暑" => (false, [1, 4, 7]),
        "白露" => (false, [9, 3, 6]),
        "秋分" => (false, [7, 1, 4]),
        "寒露" => (false, [6, 9, 3]),
        "霜降" => (false, [5, 8, 2]),
        "立冬" => (false, [6, 9, 3]),
        "小雪" => (false, [5, 8, 2]),
        "大雪" => (false, [4, 7, 1]),
        _ => return Err(invalid(format!("unknown JieQi: {term}"))),
    };
    Ok(result)
}

fn hidden_yi(xun: &str) -> Result<&'static str> {
    Ok(match xun {
        "甲子" => "戊",
        "甲戌" => "己",
        "甲申" => "庚",
        "甲午" => "辛",
        "甲辰" => "壬",
        "甲寅" => "癸",
        _ => return Err(invalid(format!("unknown xun: {xun}"))),
    })
}

fn earth_plate(yang: bool, ju: usize) -> [String; 10] {
    let stems = if yang {
        ["戊", "己", "庚", "辛", "壬", "癸", "丁", "丙", "乙"]
    } else {
        ["戊", "乙", "丙", "丁", "癸", "壬", "辛", "庚", "己"]
    };
    let mut result: [String; 10] = std::array::from_fn(|_| String::new());
    for (step, stem) in stems.iter().enumerate() {
        result[(ju + step - 1) % 9 + 1] = (*stem).into();
    }
    result
}

fn find_stem(earth: &[String; 10], stem: &str) -> Result<usize> {
    (1..=9).find(|&index| earth[index] == stem)
        .ok_or_else(|| invalid(format!("stem {stem} not found in earth plate")))
}

fn hosted(palace: usize) -> usize {
    if palace == 5 { 2 } else { palace }
}

fn gan_palace(earth: &[String; 10], stem: &str, note: Option<String>) -> Result<GanPalace> {
    let raw = find_stem(earth, stem)?;
    Ok(GanPalace {
        stem: stem.into(),
        raw_palace: Some(raw),
        palace: Some(hosted(raw)),
        note,
    })
}

fn branches(kong: &str) -> Vec<String> {
    kong.chars().map(|ch| ch.to_string()).collect()
}

fn branch_palace(branch: &str) -> Option<usize> {
    Some(match branch {
        "子" => 1,
        "丑" | "寅" => 8,
        "卯" => 3,
        "辰" | "巳" => 4,
        "午" => 9,
        "未" | "申" => 2,
        "酉" => 7,
        "戌" | "亥" => 6,
        _ => return None,
    })
}

fn kong_palaces(kong: &[String]) -> Vec<usize> {
    kong.iter()
        .filter_map(|branch| branch_palace(branch))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn yima(source: &str) -> Result<Yima> {
    let branch = match source {
        "申" | "子" | "辰" => "寅",
        "寅" | "午" | "戌" => "申",
        "亥" | "卯" | "未" => "巳",
        "巳" | "酉" | "丑" => "亥",
        _ => return Err(invalid(format!("unknown yima branch: {source}"))),
    };
    Ok(Yima {
        branch: branch.into(),
        palace: branch_palace(branch).expect("all yima branches have palaces"),
        source_branch: source.into(),
    })
}

fn element_for_stem(stem: &str) -> Option<&'static str> {
    match stem {
        "甲" | "乙" => Some("木"),
        "丙" | "丁" => Some("火"),
        "戊" | "己" => Some("土"),
        "庚" | "辛" => Some("金"),
        "壬" | "癸" => Some("水"),
        _ => None,
    }
}

fn element_for_star(star: &str) -> Option<&'static str> {
    match star {
        "天蓬" => Some("水"),
        "天任" | "天芮" | "天禽" => Some("土"),
        "天冲" | "天辅" => Some("木"),
        "天英" => Some("火"),
        "天柱" | "天心" => Some("金"),
        _ => None,
    }
}

fn element_for_door(door: &str) -> Option<&'static str> {
    match door {
        "休门" => Some("水"),
        "生门" | "死门" => Some("土"),
        "伤门" | "杜门" => Some("木"),
        "景门" => Some("火"),
        "惊门" | "开门" => Some("金"),
        _ => None,
    }
}

fn produces(a: &str, b: &str) -> bool {
    matches!((a, b), ("木", "火") | ("火", "土") | ("土", "金") | ("金", "水") | ("水", "木"))
}

fn controls(a: &str, b: &str) -> bool {
    matches!((a, b), ("木", "土") | ("土", "水") | ("水", "火") | ("火", "金") | ("金", "木"))
}

fn element_relation(a: Option<&str>, b: Option<&str>) -> Option<String> {
    let (a, b) = (a?, b?);
    Some(if a == b {
        "比和"
    } else if produces(a, b) {
        "生"
    } else if produces(b, a) {
        "被生"
    } else if controls(a, b) {
        "克"
    } else if controls(b, a) {
        "被克"
    } else {
        return None;
    }.into())
}

fn stem_relation(sky: Option<&str>, earth: Option<&str>) -> Option<String> {
    let (sky, earth) = (element_for_stem(sky?), element_for_stem(earth?));
    let (sky, earth) = (sky?, earth?);
    Some(if sky == earth {
        "比和"
    } else if produces(sky, earth) {
        "天生地"
    } else if produces(earth, sky) {
        "地生天"
    } else if controls(sky, earth) {
        "天克地"
    } else if controls(earth, sky) {
        "地克天"
    } else {
        return None;
    }.into())
}

fn palace_info(number: usize) -> (&'static str, &'static str, &'static str, &'static str) {
    match number {
        1 => ("坎宫", "北", "坎", "水"),
        2 => ("坤宫", "西南", "坤", "土"),
        3 => ("震宫", "东", "震", "木"),
        4 => ("巽宫", "东南", "巽", "木"),
        5 => ("中宫", "中", "中", "土"),
        6 => ("乾宫", "西北", "乾", "金"),
        7 => ("兑宫", "西", "兑", "金"),
        8 => ("艮宫", "东北", "艮", "土"),
        9 => ("离宫", "南", "离", "火"),
        _ => unreachable!("palace must be in 1..=9"),
    }
}

fn opposite(number: usize) -> Option<usize> {
    Some(match number {
        1 => 9, 9 => 1, 8 => 2, 2 => 8,
        3 => 7, 7 => 3, 4 => 6, 6 => 4,
        _ => return None,
    })
}

fn pattern(name: &str, palace: usize, detail: String, nature: &str) -> Pattern {
    Pattern { name: name.into(), palace, detail, nature: nature.into() }
}

/// These named patterns are structural tags for traditional interpretation,
/// not probability estimates or claims that an outcome will occur.
pub fn detect_patterns(palaces: &[QimenPalace], zhifu: &Zhifu) -> Vec<Pattern> {
    let mut found = Vec::new();
    for p in palaces.iter().filter(|p| !p.is_center) {
        for stem in &p.sky_stems {
            if ["乙", "丙", "丁"].contains(&stem.as_str())
                && matches!(p.door.as_deref(), Some("开门" | "休门" | "生门"))
            {
                found.push(pattern("三奇配吉门", p.palace, format!("{stem}+{}", p.door.as_deref().unwrap_or("")), "吉"));
            }
        }
        if p.sky_stem.is_some() && p.sky_stem == Some(p.earth_stem.clone()) {
            found.push(pattern("伏吟", p.palace, format!("天地盘同为{}", p.earth_stem), "凶"));
        }
        if let Some(star) = p.star.as_deref() {
            if let Some(home) = STARS.iter().position(|name| *name == star).map(|idx| RING[idx]) {
                if opposite(home) == Some(p.palace) {
                    found.push(pattern("反吟", p.palace, format!("{star}本宫在{home}，落对冲{}宫；天盘{}加地盘{}",
                        p.palace, p.sky_stem.as_deref().unwrap_or(""), p.earth_stem), "凶"));
                }
            }
        }
        if let Some(door) = p.door.as_deref() {
            if let Some(home) = DOORS.iter().position(|name| *name == door).map(|idx| RING[idx]) {
                if opposite(home) == Some(p.palace) {
                    found.push(pattern("门反吟", p.palace, format!("{door}本宫在{home}，落对冲{}宫", p.palace), "凶"));
                }
            }
            if let Some(door_element) = element_for_door(door) {
                if controls(door_element, &p.element) {
                    found.push(pattern("门迫", p.palace, format!("{door}({door_element})克{}({})", p.name, p.element), "凶"));
                } else if controls(&p.element, door_element) {
                    let nature = if ["开门", "休门", "生门"].contains(&door) { "凶" } else { "中" };
                    found.push(pattern("门受制", p.palace, format!("{door}({door_element})受{}({})克", p.name, p.element), nature));
                }
            }
        }
    }
    if let Some(home) = STARS.iter().position(|name| *name == zhifu.star).map(|idx| RING[idx]) {
        if home == zhifu.palace {
            found.push(pattern("值符得位", zhifu.palace, format!("{}回归本宫", zhifu.star), "吉"));
        }
    }
    found
}

/// Generate the complete nine-palace chart from a solar or lunar local civil time.
pub fn calculate(request: &QimenRequest) -> Result<QimenOutput> {
    let solar = as_solar(&request.calendar_type, &request.time_input, request.is_leap_month, false)?;
    let lunar = solar.lunar();
    let prev = lunar.prev_jie_qi().ok_or_else(|| invalid("no previous solar term"))?;
    let next = lunar.next_jie_qi();
    let active = prev.name();
    let (yang, triads) = ju_for_term(active)?;
    let day_ganzhi = lunar.day_in_gan_zhi_exact();
    let day_cycle = cycle_index(&day_ganzhi)?;
    let yuan_index = (day_cycle / 5) % 3;
    let yuan = ["上元", "中元", "下元"][yuan_index];
    let ju = triads[yuan_index];
    let earth = earth_plate(yang, ju);

    let time_ganzhi = lunar.time_in_gan_zhi();
    let (xunshou, hours_from_xunshou) = xun_from_ganzhi(&time_ganzhi)?;
    let hidden = hidden_yi(xunshou)?;
    let time_gan = time_ganzhi.chars().next().ok_or_else(|| invalid("invalid hour Ganzhi"))?.to_string();
    let visible_time_gan = if time_gan == "甲" { hidden } else { &time_gan };
    let xun_raw = find_stem(&earth, hidden)?;
    let time_raw = find_stem(&earth, visible_time_gan)?;
    let xun_palace = hosted(xun_raw);
    let time_palace = hosted(time_raw);
    let mut warnings = Vec::new();
    if time_gan == "甲" {
        warnings.push(format!("时干为甲，按旬首所遁之仪 {hidden} 入盘。"));
    }
    if xun_raw == 5 || time_raw == 5 {
        warnings.push("本规则集中宫相关判断一律寄坤处理。".into());
    }
    let step: i32 = if yang { 1 } else { -1 };
    let zhishi_raw = ((xun_raw as i32 - 1 + step * hours_from_xunshou as i32).rem_euclid(9) + 1) as usize;
    let zhishi_palace = hosted(zhishi_raw);
    if zhishi_raw == 5 {
        warnings.push("值使移宫落中五，按寄坤处理。".into());
    }

    let mut ring = RING.to_vec();
    let mut stars = STARS.to_vec();
    let mut doors = DOORS.to_vec();
    if !yang {
        ring.reverse();
        stars.reverse();
        doors.reverse();
    }
    let palace_order = rotate(&ring, &time_palace);
    let door_palace_order = rotate(&ring, &zhishi_palace);
    let xun_ring_index = RING.iter().position(|&n| n == xun_palace).expect("hosted palace in outer ring");
    let star_order = rotate(&stars, &STARS[xun_ring_index]);
    let door_order = rotate(&doors, &DOORS[xun_ring_index]);
    let outer_earth: Vec<String> = ring.iter().map(|&palace| earth[palace].clone()).collect();
    let sky_start = if xun_raw == 5 { &earth[xun_palace] } else { hidden };
    let sky_order = rotate(&outer_earth, &sky_start.to_string());
    let god_order = if yang { GODS_YANG } else { GODS_YIN };
    let god_palaces = rotate(&RING, &time_palace);

    let mut sky: [Option<String>; 10] = std::array::from_fn(|_| None);
    let mut star: [Option<&str>; 10] = [None; 10];
    let mut door: [Option<&str>; 10] = [None; 10];
    let mut god: [Option<&str>; 10] = [None; 10];
    for index in 0..8 {
        sky[palace_order[index]] = Some(sky_order[index].clone());
        star[palace_order[index]] = Some(star_order[index]);
        door[door_palace_order[index]] = Some(door_order[index]);
        god[god_palaces[index]] = Some(god_order[index]);
    }
    let zhifu = Zhifu {
        star: star[time_palace].expect("star at time palace").into(),
        palace: time_palace,
    };
    let zhishi = Zhishi {
        door: door[zhishi_palace].expect("door at shifted palace").into(),
        palace: zhishi_palace,
        raw_palace: zhishi_raw,
        hours_from_xunshou,
    };
    let tianrui_palace = (1..=9).find(|&idx| star[idx] == Some("天芮"))
        .expect("one Tianrui star");
    let center_stem = earth[5].clone();
    let center_hosting = CenterHosting {
        hosted_star: "天禽".into(),
        center_stem: center_stem.clone(),
        earth_host_palace: 2,
        sky_palace: tianrui_palace,
        follows_star: "天芮".into(),
    };

    if (solar.julian_day() - prev.solar().julian_day()).abs() <= 1.0 {
        warnings.push("当前时间距离节令起点较近，属于节气边界附近。".into());
    }
    if next.as_ref().is_some_and(|jie| (jie.solar().julian_day() - solar.julian_day()).abs() <= 1.0) {
        warnings.push("当前时间距离下一个节令较近，属于节气边界附近。".into());
    }

    let time_kongwang = branches(lunar.time_xun_kong());
    let day_kongwang = branches(lunar.day_xun_kong_exact());
    let day_xun = lunar.day_xun_exact();
    let day_stem = lunar.day_gan_exact();
    let (day_mapped, day_note) = if day_stem == "甲" {
        let substitute = hidden_yi(day_xun)?;
        (substitute, Some(format!("日干为甲，取日旬首({day_xun})所遁之仪 {substitute} 入盘")))
    } else {
        (day_stem, None)
    };
    let day_gan = gan_palace(&earth, day_mapped, day_note)?;
    let mapped_gan = |raw: &str, label: &str| -> Result<GanPalace> {
        if raw == "甲" {
            gan_palace(&earth, "戊", Some(format!("{label}为甲，取戊代入")))
        } else {
            gan_palace(&earth, raw, None)
        }
    };
    let year_gan = mapped_gan(lunar.year_gan_exact(), "年干")?;
    let month_gan = mapped_gan(lunar.month_gan_exact(), "月干")?;
    let hour_branch = lunar.time_zhi();
    let day_branch = lunar.day_zhi_exact();

    let mut palaces = Vec::with_capacity(9);
    for number in 1..=9 {
        let (name, direction, trigram, element) = palace_info(number);
        let hosts_sky = number == tianrui_palace;
        let p_sky = sky[number].clone();
        let hosted_stem = if hosts_sky { Some(center_stem.clone()) } else { None };
        let sky_stems = p_sky.iter().chain(hosted_stem.iter()).cloned().collect();
        let p_star = star[number];
        let p_door = door[number];
        let note = {
            let mut notes = Vec::new();
            if number == 5 {
                notes.push(format!("中宫地盘{center_stem}寄坤，天禽随天芮转出"));
            }
            if number == 2 {
                notes.push("地盘寄坤：承载中五地盘干".into());
            }
            if hosts_sky {
                notes.push(format!("天盘随转：天禽与中干{center_stem}随天芮落此宫"));
            }
            if notes.is_empty() { None } else { Some(notes.join("；")) }
        };
        palaces.push(QimenPalace {
            palace: number,
            name: name.into(),
            direction: direction.into(),
            trigram: trigram.into(),
            element: element.into(),
            earth_stem: earth[number].clone(),
            sky_stem: p_sky.clone(),
            sky_stems,
            stem_relation: stem_relation(p_sky.as_deref(), Some(&earth[number])),
            hosted_sky_stem: hosted_stem.clone(),
            hosted_stem_relation: stem_relation(hosted_stem.as_deref(), Some(&earth[number])),
            star: p_star.map(str::to_owned),
            hosted_star: hosts_sky.then(|| "天禽".into()),
            star_element: p_star.and_then(element_for_star).map(str::to_owned),
            star_palace_relation: element_relation(p_star.and_then(element_for_star), Some(element)),
            door: p_door.map(str::to_owned),
            door_element: p_door.and_then(element_for_door).map(str::to_owned),
            door_palace_relation: element_relation(p_door.and_then(element_for_door), Some(element)),
            god: god[number].map(str::to_owned),
            is_center: number == 5,
            hosts_center: number == 2,
            hosts_center_sky: hosts_sky,
            hosting_note: note,
        });
    }
    let mut door_index = BTreeMap::new();
    let mut star_index = BTreeMap::new();
    for palace in &palaces {
        if let Some(name) = &palace.door { door_index.insert(name.clone(), palace.palace); }
        if let Some(name) = &palace.star { star_index.insert(name.clone(), palace.palace); }
    }
    star_index.insert("天禽".into(), tianrui_palace);
    let detected_patterns = detect_patterns(&palaces, &zhifu);
    let chart = QimenChart {
        dun_type: if yang { "阳遁" } else { "阴遁" }.into(),
        yuan: yuan.into(),
        ju_number: ju,
        xunshou: xunshou.into(),
        hidden_yi: hidden.into(),
        kongwang_palaces: kong_palaces(&time_kongwang),
        kongwang: time_kongwang,
        day_kongwang_palaces: kong_palaces(&day_kongwang),
        day_kongwang,
        time_stem_visible: visible_time_gan.into(),
        day_stem: day_gan,
        year_stem: year_gan,
        month_stem: month_gan,
        hour_yima: yima(hour_branch)?,
        day_yima: yima(day_branch)?,
        zhifu,
        zhishi,
        center_hosting,
        door_index,
        star_index,
        detected_patterns,
        grid_order: GRID_ORDER,
        palaces,
    };
    let timezone = if request.location.timezone.trim().is_empty() {
        "Asia/Shanghai"
    } else {
        &request.location.timezone
    };
    Ok(QimenOutput {
        normalized_input: json!({
            "question_type": request.question_type,
            "question_goal": request.question_goal,
            "calendar_type": request.calendar_type,
            "ruleset": request.ruleset,
            "timezone": timezone,
            "country": request.location.country,
            "city": request.location.city,
            "used_now": false,
            "original_time_input": request.time_input
        }),
        calendar: json!({
            "solar": {"ymd_hms": solar.to_ymd_hms(), "timezone": timezone},
            "lunar": {
                "year": lunar.year(),
                "month": lunar.month().abs(),
                "day": lunar.day(),
                "month_text": lunar.month_in_chinese(),
                "day_text": lunar.day_in_chinese(),
                "is_leap_month": lunar.month() < 0
            },
            "jieqi": {
                "active_jie": active,
                "active_jie_started_at": prev.solar().to_ymd_hms(),
                "next_jie": next.as_ref().map(|jie| jie.name()),
                "next_jie_at": next.as_ref().map(|jie| jie.solar().to_ymd_hms())
            }
        }),
        ganzhi: json!({
            "year": lunar.year_in_gan_zhi_exact(),
            "month": lunar.month_in_gan_zhi_exact(),
            "day": day_ganzhi,
            "time": time_ganzhi,
            "day_xun_exact": day_xun,
            "day_xunkong_exact": lunar.day_xun_kong_exact(),
            "time_xun": xunshou,
            "time_xunkong": lunar.time_xun_kong()
        }),
        ruleset: json!({
            "id": request.ruleset,
            "name": "时家转盘奇门（大陆默认）",
            "timezone_default": "Asia/Shanghai",
            "dun_type_rule": "冬至到芒种用阳遁，夏至到大雪用阴遁，按当前所处二十四节气（含中气）判定。",
            "yuan_rule": "按日干支所在六十甲子序列，每 5 日一元，循环上元/中元/下元。",
            "ju_rule": "按当前节气和三元，从固定定局表取局数。",
            "zhifu_zhishi_rule": "值符随时干落宫；值使从旬首地盘宫起逐时沿九宫数序移宫（阳顺阴逆，中五寄坤），二者分别移宫。",
            "center_hosting_rule": "中宫地盘干寄坤；天禽与中干随天芮转到天芮所落之宫。",
            "yima_rule": "hour_yima 取时支三合局驿马，day_yima 取日支三合局驿马，分别输出。"
        }),
        chart,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::TimeInput;

    fn request(date: &str) -> QimenRequest {
        QimenRequest {
            question_type: String::new(),
            question_goal: String::new(),
            calendar_type: "solar".into(),
            time_input: TimeInput::Text(date.into()),
            is_leap_month: false,
            location: Location::default(),
            ruleset: default_ruleset(),
        }
    }

    #[test]
    fn earth_plate_and_yuan() {
        let yang = earth_plate(true, 3);
        assert_eq!(yang[3], "戊");
        assert_eq!(yang[5], "庚");
        assert_eq!(yang[7], "壬");
        let yin = earth_plate(false, 5);
        assert_eq!(yin[5], "戊");
        assert_eq!(yin[1], "壬");
        assert_eq!(yin[9], "癸");
        assert_eq!(ju_for_term("春分").unwrap(), (true, [3, 9, 6]));
        assert_eq!(ju_for_term("寒露").unwrap(), (false, [6, 9, 3]));
    }

    #[test]
    fn golden_spring_2026() {
        let output = calculate(&request("2026-03-24 10:30")).unwrap();
        let chart = output.chart;
        assert_eq!((&chart.dun_type[..], &chart.yuan[..], chart.ju_number), ("阳遁", "上元", 3));
        assert_eq!(chart.xunshou, "甲辰");
        assert_eq!(chart.hidden_yi, "壬");
        assert_eq!(chart.zhifu, Zhifu { star: "天柱".into(), palace: 2 });
        assert_eq!(chart.zhishi.door, "惊门");
        assert_eq!(chart.zhishi.palace, 8);
        assert_eq!(chart.kongwang_palaces, [3, 8]);
        assert_eq!(chart.palaces[4].earth_stem, "庚");
    }

    #[test]
    fn golden_summer_2026() {
        let chart = calculate(&request("2026-07-15 14:00")).unwrap().chart;
        assert_eq!((&chart.dun_type[..], &chart.yuan[..], chart.ju_number), ("阴遁", "下元", 5));
        assert_eq!(chart.zhifu.star, "天辅");
        assert_eq!(chart.zhifu.palace, 9);
        assert_eq!(chart.zhishi.door, "杜门");
        assert_eq!(chart.zhishi.palace, 4);
        assert_eq!(chart.zhishi.hours_from_xunshou, 9);
    }

    #[test]
    fn regression_separate_door_and_star_october() {
        let chart = calculate(&request("2026-10-12 02:03:53")).unwrap().chart;
        assert_eq!(chart.ju_number, 3);
        assert_eq!(chart.zhifu, Zhifu { star: "天冲".into(), palace: 4 });
        assert_eq!(chart.zhishi.palace, 2);
        let doors: Vec<Option<&str>> = chart.palaces.iter().map(|palace| palace.door.as_deref()).collect();
        assert_eq!(doors, [
            Some("死门"), Some("伤门"), Some("开门"), Some("休门"), None,
            Some("景门"), Some("杜门"), Some("惊门"), Some("生门")
        ]);
        assert!(!chart.detected_patterns.iter().any(|p| p.name == "三奇配吉门" && p.palace == 1));
    }

    #[test]
    fn no_centermost_door_or_god() {
        let chart = calculate(&request("2026-03-24 10:30")).unwrap().chart;
        assert_eq!(chart.palaces.len(), 9);
        assert!(chart.palaces[4].star.is_none());
        assert!(chart.palaces[4].door.is_none());
        assert!(chart.palaces[4].god.is_none());
        assert_eq!(chart.center_hosting.earth_host_palace, 2);
        assert_eq!(chart.star_index["天禽"], chart.center_hosting.sky_palace);
    }

    #[test]
    fn invalid_calendar_input_fails_closed() {
        assert!(calculate(&request("2026-02-30 14:00")).is_err());
    }
}
