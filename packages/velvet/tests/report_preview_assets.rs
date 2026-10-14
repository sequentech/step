// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::types::ceremonies::CountingAlgType;
use serde_json::Value;
use velvet::pipes::generate_reports::TemplateData;

#[test]
fn tally_report_preview_assets_deserialize() {
    for (name, data) in [
        (
            "initialization_report",
            include_str!("../../../.devcontainer/minio/public-assets/initialization_report.json"),
        ),
        (
            "electoral_results",
            include_str!("../../../.devcontainer/minio/public-assets/electoral_results.json"),
        ),
    ] {
        serde_json::from_str::<TemplateData>(data)
            .unwrap_or_else(|error| panic!("{name} preview cannot deserialize: {error}"));
    }
}

fn check_algorithms(value: &Value, path: &str) -> usize {
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| {
                let path = format!("{path}.{key}");
                if key == "counting_algorithm" && !value.is_null() {
                    serde_json::from_value::<CountingAlgType>(value.clone())
                        .unwrap_or_else(|error| panic!("{path}: {error}"));
                    1
                } else {
                    check_algorithms(value, &path)
                }
            })
            .sum(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| check_algorithms(value, &format!("{path}[{index}]")))
            .sum(),
        _ => 0,
    }
}

#[test]
fn report_preview_assets_use_serializable_counting_algorithms() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.devcontainer/minio/public-assets");
    let mut checked = 0;
    for entry in std::fs::read_dir(assets).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let data: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        checked += check_algorithms(&data, &path.display().to_string());
    }
    assert!(checked > 0, "no preview counting algorithms were checked");
}
