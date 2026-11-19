//! Four pillars primitive; interpretive BaZi rules remain in upstream bazi/SKILL.md.

use lunar_rs::Solar;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::calendar::{TimeInput, as_solar};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaziRequest {
    #[serde(default = "default_calendar")]
    pub calendar_type: String,
    pub birth_time: TimeInput,
    #[serde(default)]
    pub is_leap_month: bool,
}

fn default_calendar() -> String {
    "solar".into()
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

pub fn calculate(request: &BaziRequest) -> Result<FourPillars> {
    let solar: Solar = as_solar(
        &request.calendar_type,
        &request.birth_time,
        request.is_leap_month,
        true,
    )?;
    let lunar = solar.lunar();
    Ok(FourPillars {
        year: lunar.year_in_gan_zhi_exact(),
        month: lunar.month_in_gan_zhi_exact(),
        day: lunar.day_in_gan_zhi_exact(),
        hour: lunar.time_in_gan_zhi(),
        solar_datetime: solar.to_ymd_hms(),
        calculation: "exact JieQi year/month, exact day pillar, civil local hour".into(),
    })
}
