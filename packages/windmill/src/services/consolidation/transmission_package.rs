// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::s3::{download_s3_file_to_string, get_public_asset_file_path};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::ballot::Annotations;
use sequent_core::services::reports;
use sequent_core::types::date_time::TimeZone;
use serde_json::{Map, Value};
use std::env;
use velvet::pipes::{do_tally::ContestResult, generate_reports::ReportData};

use super::eml_generator::render_eml_file;

pub async fn create_transmission_package(
    tally_id: i64,
    transaction_id: i64,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &Annotations,
    election_annotations: &Annotations,
    report: &ReportData,
) -> Result<()> {
    let eml_data = render_eml_file(
        tally_id,
        transaction_id,
        time_zone,
        date_time,
        &election_event_annotations,
        &election_annotations,
        &report,
    )?;
    let mut variables_map: Map<String, Value> = Map::new();
    variables_map.insert("data".to_string(), serde_json::to_value(eml_data)?);
    let template_path = env::var("PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE")?;
    let s3_template_url = get_public_asset_file_path(&template_path)
        .with_context(|| "Error fetching get_minio_url")?;
    let template_string = download_s3_file_to_string(&s3_template_url).await?;
    // render handlebars template
    let render = reports::render_template_text(&template_string, variables_map)
        .map_err(|err| anyhow!("{}", err))?;

    Ok(())
}
