// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, ensure, Context, Result};
use serde::Serialize;
use std::{collections::HashSet, io::Write};

/// Tally exports store JSON values in CSV cells, in the explicit header order.
/// Refuse schema drift rather than silently mislabelling or dropping a field.
pub(super) fn write_json_record<W: Write, T: Serialize>(
    writer: &mut csv::Writer<W>,
    headers: &[String],
    record: T,
) -> Result<()> {
    let value = serde_json::to_value(record)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("Tally export record must be a JSON object"))?;
    ensure!(
        object.len() == headers.len()
            && headers.iter().collect::<HashSet<_>>().len() == headers.len(),
        "Tally export fields differ from its CSV header"
    );
    let values = headers
        .iter()
        .map(|name| {
            object
                .get(name)
                .map(serde_json::Value::to_string)
                .ok_or_else(|| anyhow!("Tally export field missing: {name}"))
        })
        .collect::<Result<Vec<_>>>()?;
    writer
        .write_record(&values)
        .context("Error writing tally export CSV record")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn headers_determine_cell_order_and_json_values_round_trip() {
        let headers = ["id", "votes", "annotations", "missing"].map(str::to_owned);
        let row = json!({"annotations": {"note": "comma, newline\nquote\""},
                         "votes": 42, "missing": null, "id": "result-1"});
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer.write_record(&headers).unwrap();
        write_json_record(&mut writer, &headers, &row).unwrap();
        let bytes = writer.into_inner().unwrap();
        let mut reader = csv::Reader::from_reader(bytes.as_slice());
        assert_eq!(
            reader.headers().unwrap().iter().collect::<Vec<_>>(),
            headers
        );
        let parsed = reader.records().next().unwrap().unwrap();
        for (name, cell) in headers.iter().zip(parsed.iter()) {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(cell).unwrap(),
                row[name]
            );
        }
        assert!(reader.records().next().is_none());
    }

    #[test]
    fn missing_extra_and_duplicate_columns_are_rejected_before_writing() {
        for (headers, row) in [
            (vec!["id"], json!({"id": "r", "new_field": 1})),
            (vec!["id", "absent"], json!({"id": "r", "votes": 1})),
            (vec!["id", "id"], json!({"id": "r", "votes": 1})),
            (vec!["id"], json!(["r"])),
        ] {
            let headers = headers.into_iter().map(str::to_owned).collect::<Vec<_>>();
            let mut writer = csv::Writer::from_writer(Vec::new());
            assert!(write_json_record(&mut writer, &headers, row).is_err());
            assert!(writer.into_inner().unwrap().is_empty());
        }
    }

    #[test]
    fn output_failure_reaches_the_export_caller() {
        struct FailedOutput;
        impl Write for FailedOutput {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("simulated full disk"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = csv::WriterBuilder::new()
            .buffer_capacity(1)
            .from_writer(FailedOutput);
        let error =
            write_json_record(&mut writer, &["id".to_owned()], json!({"id": "r"})).unwrap_err();
        assert!(format!("{error:#}").contains("simulated full disk"));
    }
}
