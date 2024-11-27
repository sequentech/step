use sequent_core::types::hasura::core::ApplicantAttribte;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use tracing::{event, instrument, Level};
use tokio_postgres::row::Row;
use uuid::Uuid;
pub struct ApplicationAttributeWrapper(pub ApplicantAttribte);


impl TryFrom<Row> for ApplicationAttributeWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(ApplicationAttributeWrapper(ApplicantAttribte {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            created_at: item.get("created_at"),
            updated_at: item.get("updated_at"),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            application_id: item.try_get::<_, Uuid>("application_id")?.to_string(),
            applicant_attribute_name: item.try_get("applicant_attribute_name")?,
            applicant_attribute_value: item.try_get("applicant_attribute_value")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_applicant_attribute(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    application_id: &str,
    applicant_attribute_name: &str,
    applicant_attribute_value: String,
) -> Result<()> {
    let tenant_id = Uuid::parse_str(tenant_id)?;
    let application_id = Uuid::parse_str(application_id)?;

    let statement = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.applicant_attributes
            (
                tenant_id,
                application_id,
                applicant_attribute_name,
                applicant_attribute_value
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            );
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the insert applicant attribute query: {err}"))?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &tenant_id,
                &application_id,
                &applicant_attribute_name,
                &applicant_attribute_value,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting applicant attribute: {err}"))?;

    Ok(())
}
#[instrument(err, skip_all)]
pub async fn get_applicant_attributes_by_application_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    application_id: &str
) -> Result<Vec<Row>> {
    let tenant_id = Uuid::parse_str(tenant_id)?;
    let application_id = Uuid::parse_str(application_id)?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT applicant_attribute_name, applicant_attribute_value
            FROM sequent_backend.applicant_attributes
            WHERE tenant_id = $1 AND application_id = $2;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the select applicant attributes query: {err}"))?;

    let rows = hasura_transaction
        .query(&statement, &[&tenant_id, &application_id])
        .await
        .map_err(|err| anyhow!("Error querying applicant attributes: {err}"))?;

    Ok(rows)
}