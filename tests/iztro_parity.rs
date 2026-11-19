//! Independent Ziwei reference data supplied by the upstream fork.
//!
//! The JSON corpus was generated from iztro 2.6.2. It remains an oracle,
//! not a native dependency. This test executes the Rust library only.

use std::collections::BTreeMap;

use numerologist_core::ziwei::{ZiweiRequest, calculate};
use serde_json::{Value, json};

#[test]
fn all_iztro_2_6_2_reference_cases() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../ziwei-doushu/scripts/tests/fixtures/iztro_cases.json"
    ))
    .expect("valid pinned iztro corpus");
    assert_eq!(corpus["generator"], "iztro@2.6.2");

    let cases = corpus["cases"].as_array().expect("test cases");
    assert!(!cases.is_empty());
    for (index, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let source = &case["iztro"];
        let label = format!(
            "iztro case #{index} birth={} gender={}",
            input["birth_time"], input["gender"]
        );
        let request: ZiweiRequest = serde_json::from_value(json!({
            "calendar_type": "solar",
            "birth_time": input["birth_time"],
            "gender": input["gender"],
            "leap_month_rule": "split_15"
        }))
        .expect("corpus input shape");
        let result = calculate(&request).unwrap_or_else(|err| panic!("{label}: {err}"));
        let chart = result.chart;

        assert_eq!(chart.ming_gong.branch, source["ming"].as_str().unwrap(), "{label}: ming");
        assert_eq!(chart.shen_gong.branch, source["shen"].as_str().unwrap(), "{label}: shen");
        assert_eq!(chart.wuxing_ju.name, source["ju"].as_str().unwrap(), "{label}: ju");
        assert_eq!(chart.ming_zhu, source["ming_zhu"].as_str().unwrap(), "{label}: ming_zhu");
        assert_eq!(chart.shen_zhu, source["shen_zhu"].as_str().unwrap(), "{label}: shen_zhu");

        let expected_stars: BTreeMap<String, String> = serde_json::from_value(source["stars"].clone())
            .expect("expected star positions");
        for (star, expected_branch) in expected_stars {
            assert_eq!(
                chart.star_index.get(&star).map(String::as_str),
                Some(expected_branch.as_str()),
                "{label}: star={star}"
            );
        }

        let expected_sihua: BTreeMap<String, String> =
            serde_json::from_value(source["sihua"].clone()).expect("expected Sihua");
        let actual_sihua: BTreeMap<String, String> = chart
            .year_sihua.iter()
            .map(|item| {
                (
                    item.star.clone(),
                    item.kind.chars().nth(1).expect("Sihua type 化X").to_string()
                )
            })
            .collect();
        assert_eq!(actual_sihua, expected_sihua, "{label}: Sihua");

        let by_branch: BTreeMap<String, _> = chart.palaces
            .iter().map(|palace| (palace.branch.clone(), palace)).collect();
        let stems = numerologist_core::calendar::BRANCHES.iter()
            .map(|branch| by_branch[*branch].stem.as_str())
            .collect::<String>();
        assert_eq!(stems, source["stems"].as_str().unwrap(), "{label}: palace stems");
        let decadal_starts: Vec<usize> = numerologist_core::calendar::BRANCHES.iter()
            .map(|branch| by_branch[*branch].decadal.start_age)
            .collect();
        let reference_starts: Vec<usize> = serde_json::from_value(source["decadal_starts"].clone())
            .expect("expected decadal starts");
        assert_eq!(decadal_starts, reference_starts, "{label}: decadal starts");

        let brightness: BTreeMap<String, String> = chart.palaces.iter()
            .flat_map(|palace| palace.major_stars.iter())
            .map(|star| (star.name.clone(), star.brightness.clone()))
            .collect();
        let expected_brightness: BTreeMap<String, String> =
            serde_json::from_value(source["brightness"].clone()).expect("brightness table");
        assert_eq!(brightness, expected_brightness, "{label}: brightness");
    }
}
