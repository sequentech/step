// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The check before an election event import reads anything: a signed
//! configuration package is verified against the tenant's trust, its
//! revision must be newer than the last one imported, and only then is its
//! importable archive handed to the importer. An unsigned file is refused
//! unless the tenant's setting allows it; a tenant without the setting
//! imports only signed packages.

use std::collections::HashMap;
use std::io::Write;

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::import_problems::{allowed_votes_raised, allows_more_votes};
use sequent_core::election_config::manifest::open_package;
use sequent_core::election_config::package_verify::{
    admit_at, already_imported, crls_from_pem, freshness, importable_member, Admission,
    ConfigurationSigning, Freshness, VerifiedPackage, CONFIGURATION_SIGNING_SETTING,
};
use sequent_core::election_config::{Rejected, ValidationReport};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::Election;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

use crate::postgres::configuration_packages;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::electoral_log::{ElectoralLog, ElectoralLogAdminContext};
use crate::services::electoral_log_board::get_event_board;
use crate::services::import::rejection::reject;
use electoral_log::messages::newtypes::{
    ConfigurationDesignDigest, ConfigurationPackageAction, ConfigurationPackageDetails,
};

const WHAT: &str = "configuration package";

/// The tenant's `configuration_signing` setting, if it has one.
pub async fn tenant_setting(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Option<ConfigurationSigning>> {
    let tenant = get_tenant_by_id(hasura_transaction, tenant_id).await?;
    tenant
        .settings
        .as_ref()
        .and_then(|settings| settings.get(CONFIGURATION_SIGNING_SETTING))
        .map(|value| {
            serde_json::from_value(value.clone()).with_context(|| {
                format!("the tenant's {CONFIGURATION_SIGNING_SETTING} is malformed")
            })
        })
        .transpose()
}

/// Admits the file at `temp_file`: returns the file the importer reads (the
/// package's importable archive, for a package) and the verified package.
#[instrument(skip(hasura_transaction, temp_file), err)]
pub async fn admit_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    temp_file: NamedTempFile,
) -> Result<(NamedTempFile, Option<Box<VerifiedPackage>>)> {
    let bytes = std::fs::read(temp_file.path()).context("could not read the uploaded file")?;
    // One package import per tenant at a time, from here until the importer
    // records it: a concurrent one would not see this one's revision or the
    // revocation lists it brings. An unsigned file reads neither.
    if open_package(&bytes).is_ok_and(|opened| opened.is_signed()) {
        configuration_packages::lock_imports(hasura_transaction, tenant_id).await?;
    }
    let setting = tenant_setting(hasura_transaction, tenant_id).await?;
    let lists = configuration_packages::revocation_lists(hasura_transaction, tenant_id).await?;

    let verified = match admit_at(&bytes, setting.as_ref(), lists, chrono::Utc::now()) {
        Ok(Admission::Unsigned) => return Ok((temp_file, None)),
        Ok(Admission::Verified(verified)) => verified,
        Err(report) => return Err(anyhow::Error::new(Rejected::new(WHAT, report))),
    };

    let configuration = &verified.manifest.configuration;
    let last = configuration_packages::last_import(
        hasura_transaction,
        tenant_id,
        &configuration.external_id,
    )
    .await?;
    match freshness(&verified, last.as_ref()) {
        Ok(Freshness::New) => {}
        Ok(Freshness::AlreadyImported) => {
            return Err(reject(WHAT, already_imported(configuration.revision)));
        }
        Err(problem) => return Err(reject(WHAT, problem)),
    }

    let archive = importable_member(&verified).map_err(|problem| reject(WHAT, problem))?;
    let mut importable = NamedTempFile::new().context("could not create a temporary file")?;
    importable.write_all(archive)?;
    importable.flush()?;
    info!(
        external_id = %configuration.external_id,
        revision = configuration.revision,
        manifest_sha256 = %verified.manifest_sha256,
        "verified configuration package"
    );
    Ok((importable, Some(verified)))
}

/// Refuses a package that gives an election more votes per voter than the
/// election with its external id has now in the event the configuration's
/// last revision was imported into: a new revision keeps or lowers that number.
#[instrument(skip(hasura_transaction, package, elections), err)]
pub async fn check_allowed_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    package: &VerifiedPackage,
    elections: &[Election],
) -> Result<()> {
    let rows = hasura_transaction
        .query(
            "SELECT election.external_id, election.num_allowed_revotes
             FROM sequent_backend.election AS election
             JOIN (SELECT election_event_id FROM sequent_backend.configuration_package
                   WHERE tenant_id = $1 AND external_id = $2
                   ORDER BY revision DESC LIMIT 1) AS last
               ON election.election_event_id = last.election_event_id
             WHERE election.tenant_id = $1 AND election.external_id IS NOT NULL",
            &[
                &parse_uuid_v4(tenant_id)?,
                &package.manifest.configuration.external_id,
            ],
        )
        .await
        .context("could not read the allowed votes of the last imported revision")?;
    let current = rows
        .iter()
        .map(|row| -> Result<(String, Option<i64>)> {
            Ok((
                row.try_get::<_, String>(0)?,
                row.try_get::<_, Option<i32>>(1)?.map(i64::from),
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let report = raised_allowed_votes(elections, &current);
    if report.has_errors() {
        return Err(anyhow::Error::new(Rejected::new(
            "election event bundle",
            report,
        )));
    }
    Ok(())
}

/// A problem for each of `elections` that allows more votes per voter than
/// `current` gives for its external id.
fn raised_allowed_votes(
    elections: &[Election],
    current: &HashMap<String, Option<i64>>,
) -> ValidationReport {
    let mut report = ValidationReport::default();
    for (index, election) in elections.iter().enumerate() {
        let Some(external_id) = election.external_id.as_deref() else {
            continue;
        };
        if let Some(current) = current.get(external_id) {
            if allows_more_votes(election.num_allowed_revotes, *current) {
                report.push(allowed_votes_raised(
                    index,
                    external_id,
                    election.num_allowed_revotes,
                    *current,
                ));
            }
        }
    }
    report
}

/// The electoral log entry of an import, posted to the new event's board.
pub async fn log_import(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    package: &VerifiedPackage,
    initiator: Option<&ElectoralLogAdminContext>,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log
        .post_configuration_package(
            election_event_id.to_string(),
            ConfigurationPackageDetails {
                action: ConfigurationPackageAction::Imported,
                external_id: package.manifest.configuration.external_id.clone(),
                revision: package.manifest.configuration.revision,
                manifest_sha256: package.manifest_sha256.clone(),
                design_digests: package
                    .manifest
                    .content
                    .ballot_designs
                    .iter()
                    .map(|design| ConfigurationDesignDigest {
                        area: design.area.clone(),
                        election: design.election.clone(),
                        sha256: design.sha256.clone(),
                    })
                    .collect(),
            },
            initiator.map(|initiator| initiator.user_id.clone()),
            initiator.and_then(|initiator| initiator.username.clone()),
        )
        .await
}

/// Records an imported package and keeps the revocation lists it carried,
/// so a revoked key's packages are refused from now on.
pub async fn record(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    package: &VerifiedPackage,
) -> Result<()> {
    configuration_packages::record_import(
        hasura_transaction,
        tenant_id,
        election_event_id,
        package,
    )
    .await?;
    let mut lists = Vec::new();
    for pem in &package.manifest.revocation_lists {
        lists.extend(crls_from_pem(pem).map_err(|error| anyhow!(error))?);
    }
    configuration_packages::remember_revocation_lists(hasura_transaction, tenant_id, &lists).await
}

/// The checksum typed with an upload, where it still has something to check.
///
/// A verified package was checked file by file against its signed manifest,
/// so a hand-typed checksum of the upload has nothing to add.
pub fn typed_checksum<P>(typed: Option<String>, package: Option<&P>) -> Option<String> {
    match package {
        Some(_) => None,
        None => typed,
    }
}

#[cfg(test)]
mod tests {
    use super::{raised_allowed_votes, typed_checksum};
    use sequent_core::types::hasura::core::Election;
    use std::collections::HashMap;

    fn election(external_id: Option<&str>, num_allowed_revotes: Option<i64>) -> Election {
        serde_json::from_value(serde_json::json!({
            "id": "e0000000-0000-4000-8000-000000000001",
            "tenant_id": "e0000000-0000-4000-8000-000000000002",
            "election_event_id": "e0000000-0000-4000-8000-000000000003",
            "external_id": external_id,
            "num_allowed_revotes": num_allowed_revotes,
        }))
        .expect("an election")
    }

    #[test]
    fn a_new_revision_keeps_or_lowers_the_votes_of_each_election() {
        let current = HashMap::from([
            ("president".to_string(), Some(2)),
            ("council".to_string(), None),
            ("board".to_string(), Some(0)),
        ]);
        let kept = [
            election(Some("president"), Some(1)),
            election(Some("council"), Some(1)),
            election(Some("board"), Some(3)),
            election(Some("new"), Some(0)),
            election(None, Some(0)),
        ];
        assert!(raised_allowed_votes(&kept, &current).problems.is_empty());

        let raised = [
            election(Some("president"), Some(0)),
            election(Some("council"), Some(2)),
            election(Some("board"), Some(0)),
        ];
        let report = raised_allowed_votes(&raised, &current);
        let paths: Vec<&str> = report
            .problems
            .iter()
            .map(|problem| problem.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "elections[0].num_allowed_revotes",
                "elections[1].num_allowed_revotes"
            ]
        );
        assert!(report.has_errors());
    }

    #[test]
    fn a_plain_upload_keeps_the_checksum_typed_with_it() {
        assert_eq!(
            typed_checksum::<()>(Some("abc".to_string()), None),
            Some("abc".to_string())
        );
        assert_eq!(typed_checksum::<()>(None, None), None);
    }

    #[test]
    fn a_verified_package_needs_no_typed_checksum() {
        assert_eq!(typed_checksum(Some("abc".to_string()), Some(&())), None);
        assert_eq!(typed_checksum(None, Some(&())), None);
    }
}
