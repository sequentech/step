// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use tracing::instrument;

use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::documents::upload_and_return_document;
use crate::services::reports::utils::get_number_format_policy;
use crate::tasks::render_report::{FormatType, RenderTemplateBody};
use sequent_core::types::number_format::{NumberFormatPolicy, NUMBER_FORMAT_POLICY_VARIABLE};
use sequent_core::util::temp_path::write_into_named_temp_file;

#[instrument(err, skip(hasura_transaction))]
pub async fn render_report_task(
    hasura_transaction: &Transaction<'_>,
    input: RenderTemplateBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let tenant = get_tenant_by_id(hasura_transaction, &tenant_id).await?;
    let election_event =
        get_election_event_by_id_if_exist(hasura_transaction, &tenant_id, &election_event_id)
            .await
            .context("Error reading the election event's number format")?;

    let variables_map = report_variables(
        input.variables.clone(),
        &tenant.slug,
        election_event.as_ref().and_then(get_number_format_policy),
    );

    // render handlebars template
    let render = reports::render_template_text(input.template.as_str(), variables_map)
        .map_err(|err| anyhow!("{}", err))?;

    // if output format is text/html, just return that
    if FormatType::TEXT == input.format {
        let (_temp_path, temp_path_string, file_size) =
            write_into_named_temp_file(&render.into_bytes(), "reports-", ".html")
                .with_context(|| "Error writing to file")?;
        upload_and_return_document(
            &hasura_transaction,
            &temp_path_string,
            file_size,
            "text/plain",
            &tenant_id,
            Some(election_event_id),
            &input.name,
            None,
            false,
        )
        .await?;
    } else {
        let bytes = pdf::PdfRenderer::render_pdf(render, None)
            .await
            .with_context(|| "Error converting html to pdf format")?;
        let (_temp_path, temp_path_string, file_size) =
            write_into_named_temp_file(&bytes, "reports-", ".html")
                .with_context(|| "Error writing to file")?;

        let _document = upload_and_return_document(
            &hasura_transaction,
            &temp_path_string,
            file_size,
            "application/pdf",
            &tenant_id,
            Some(election_event_id),
            &input.name,
            None,
            false,
        )
        .await?;
    }
    Ok(())
}

/// The variables a report's template is rendered with: the request's, plus
/// the tenant's slug as `username` and the election event's number format for
/// its figures, unless the request sets them.
fn report_variables(
    mut variables: Map<String, Value>,
    username: &str,
    number_format_policy: Option<NumberFormatPolicy>,
) -> Map<String, Value> {
    if !variables.contains_key("username") {
        variables.insert("username".to_string(), json!(username));
    }
    if let Some(policy) = number_format_policy {
        if !variables.contains_key(NUMBER_FORMAT_POLICY_VARIABLE) {
            variables.insert(NUMBER_FORMAT_POLICY_VARIABLE.to_string(), json!(policy));
        }
    }
    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(variables: Value) -> Map<String, Value> {
        variables.as_object().cloned().unwrap()
    }

    #[test]
    fn report_variables_add_the_tenant_and_the_event_number_format() {
        let variables = report_variables(
            request(json!({"votes": 1234567})),
            "tenant",
            Some(NumberFormatPolicy::PeriodComma),
        );
        assert_eq!(
            variables,
            request(json!({
                "votes": 1234567,
                "username": "tenant",
                "number_format_policy": "period-comma",
            }))
        );
    }

    #[test]
    fn report_variables_keep_the_ones_the_request_sets() {
        let sent = request(json!({
            "username": "someone",
            "number_format_policy": "space-comma",
        }));
        let variables = report_variables(
            sent.clone(),
            "tenant",
            Some(NumberFormatPolicy::PeriodComma),
        );
        assert_eq!(variables, sent);
    }

    #[test]
    fn report_variables_name_no_number_format_for_an_event_without_one() {
        let variables = report_variables(request(json!({"votes": 1234567})), "tenant", None);
        assert!(!variables.contains_key(NUMBER_FORMAT_POLICY_VARIABLE));
    }

    #[test]
    fn report_figures_use_the_event_number_format() {
        let render = |policy| {
            reports::render_template_text(
                "{{format_u64 votes}}",
                report_variables(request(json!({"votes": 1234567})), "tenant", policy),
            )
            .unwrap()
        };
        assert_eq!(render(Some(NumberFormatPolicy::PeriodComma)), "1.234.567");
        assert_eq!(render(None), "1,234,567");
    }
}
