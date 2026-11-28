// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A package's message templates applied against PostgreSQL: each stored
//! under its alias with every channel's part, never twice, however often the
//! package is applied. Synthetic values only.

#[path = "support/schema.rs"]
mod schema;

use sequent_core::election_config::package_templates::{read, COLUMNS};
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::import::configuration_package_templates::apply;

fn package(rows: &[(&str, Value)]) -> Vec<u8> {
    let mut writer = csv::Writer::from_writer(vec![]);
    writer.write_record(COLUMNS).unwrap();
    for (alias, document) in rows {
        writer
            .write_record([
                alias,
                "Voting reminder",
                "CREDENTIALS",
                "EMAIL",
                &document.to_string(),
                "",
            ])
            .unwrap();
    }
    writer.into_inner().unwrap()
}

fn reminder(text: &str) -> Value {
    json!({
        "email": {"subject":"Voting reminder","plaintext_body":text,"html_body":text},
        "whatsapp": {"message":text,"parameters":["{{user.first_name}}"],
            "provider_template":"step_notice","provider_language":"en",
            "buttons":[{"type":"URL","parameter":"{{vote_url}}"}]},
        "viber": {"message":text},
        "selected_methods": {"EMAIL":true,"WHATSAPP":true,"VIBER":true}
    })
}

async fn stored(tx: &deadpool_postgres::Transaction<'_>, tenant: &Uuid) -> Vec<(String, Value)> {
    tx.query(
        "SELECT alias, template FROM sequent_backend.template WHERE tenant_id = $1 ORDER BY alias, id",
        &[tenant],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get("alias"), row.get("template")))
    .collect()
}

#[tokio::test]
async fn package_templates_are_stored_by_alias_and_never_twice() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tenant = Uuid::parse_str("00000000-0000-4000-8000-0000000002a1").unwrap();
    let tx = client.transaction().await.unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    // Two earlier hand imports of the same template.
    for _ in 0..2 {
        tx.execute(
            "INSERT INTO sequent_backend.template (alias, tenant_id, template, created_by, communication_method, type)
             VALUES ('old-notice', $1, '{}'::jsonb, 'admin', 'EMAIL', 'CREDENTIALS')",
            &[&tenant],
        )
        .await
        .unwrap();
    }

    let first = read(&package(&[
        ("reminder", reminder("Vote now")),
        ("old-notice", reminder("Polls close soon")),
    ]))
    .unwrap();
    let changes = apply(&tx, &tenant, &first).await.unwrap();
    assert_eq!(changes[0]["action"], "create");
    assert_eq!(changes[1]["action"], "replace");
    let rows = stored(&tx, &tenant).await;
    assert_eq!(rows.len(), 3);
    let reminder_row = &rows
        .iter()
        .find(|(alias, _)| alias == "reminder")
        .unwrap()
        .1;
    assert_eq!(
        reminder_row["whatsapp"]["buttons"][0]["parameter"],
        "{{vote_url}}"
    );
    assert_eq!(reminder_row["viber"]["message"], "Vote now");
    assert!(rows
        .iter()
        .filter(|(alias, _)| alias == "old-notice")
        .all(|(_, template)| template["viber"]["message"] == "Polls close soon"));

    // Applying the same package again changes nothing and adds nothing.
    let again = apply(&tx, &tenant, &first).await.unwrap();
    assert!(again
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["action"] == "unchanged"));
    assert_eq!(stored(&tx, &tenant).await, rows);

    // A revised package replaces its template in place.
    let revised = read(&package(&[("reminder", reminder("Vote today"))])).unwrap();
    assert_eq!(
        apply(&tx, &tenant, &revised).await.unwrap()[0]["action"],
        "replace"
    );
    let rows = stored(&tx, &tenant).await;
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .find(|(alias, _)| alias == "reminder")
            .unwrap()
            .1["viber"]["message"],
        "Vote today"
    );
    tx.rollback().await.unwrap();
}
