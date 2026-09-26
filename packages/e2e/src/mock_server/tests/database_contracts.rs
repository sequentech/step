// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[macro_use]
extern crate rocket;
#[path = "../src/routes/scanovate.rs"]
mod scanovate;
#[path = "../src/services/mod.rs"]
mod services;
#[path = "../src/types/mod.rs"]
mod types;

use services::user::{get_users_from_db, load_users, random_user_by_country};
use std::sync::Mutex;
static DATABASE: Mutex<()> = Mutex::new(());
struct WorkingDirectory(std::path::PathBuf);
impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).unwrap();
    }
}
fn database() -> (tempfile::TempDir, WorkingDirectory) {
    let temp = tempfile::tempdir().unwrap();
    let guard = WorkingDirectory(std::env::current_dir().unwrap());
    std::env::set_current_dir(temp.path()).unwrap();
    (temp, guard)
}
fn csv() {
    // Distinct values catch swapped positional columns; no real voter data.
    std::fs::write("synthetic.csv", "first,last,x,y,middle,birth,embassy,country,a,b,c,d,number,type\n Ada , Lovelace ,x,y, M ,2000-02-29, Madrid , XX ,a,b,c,d, SYN-17 , TestCard \n").unwrap();
}

#[tokio::test]
async fn csv_mapping_and_http_results_preserve_the_selected_synthetic_identity() {
    let _lock = DATABASE.lock().unwrap_or_else(|error| error.into_inner());
    let (_temp, _cwd) = database();
    csv();
    assert_eq!(load_users("synthetic.csv").unwrap(), 1);
    let users = get_users_from_db().unwrap();
    assert_eq!(users.len(), 1);
    let row = &users[0];
    assert!(uuid::Uuid::parse_str(&row.id).is_ok());
    assert_eq!(
        serde_json::to_value(row).unwrap(),
        serde_json::json!({
            "id":row.id, "firstName":"Ada", "lastName":"Lovelace", "middleName":"M",
            "dateOfBirth":"29/02/2000", "embassy":"Madrid", "country":"XX",
            "idCardNumber":"SYN-17", "idCardType":"TestCard"
        })
    );
    assert_eq!(random_user_by_country("XX").unwrap().unwrap().id, row.id);
    assert!(random_user_by_country("' OR 1=1 --").unwrap().is_none());
    let client = rocket::local::asynchronous::Client::tracked(
        rocket::build()
            .manage(scanovate::MockSessions::default())
            .mount(
                "/",
                routes![
                    scanovate::auth_token,
                    scanovate::flow_link,
                    scanovate::flow_complete,
                    scanovate::session_token,
                    scanovate::results_with_image_names
                ],
            ),
    )
    .await
    .unwrap();
    let token: serde_json::Value = client
        .post("/auth/token")
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    assert!(uuid::Uuid::parse_str(token["access_token"].as_str().unwrap()).is_ok());

    let results = |country: &'static str, process_id: &'static str| {
        let client = &client;
        async move {
            let link: serde_json::Value = client
                .post("/flow/v3/link")
                .json(&serde_json::json!({
                    "flow_id": 7,
                    "id_number": "TYPED-1",
                    "identifier_id": process_id,
                    "redirect_url": "http://keycloak/return?code=1",
                    "params": { "country": country }
                }))
                .dispatch()
                .await
                .into_json::<serde_json::Value>()
                .await
                .unwrap();
            assert!(link["data"]
                .as_str()
                .unwrap()
                .ends_with(&format!("process_id={process_id}")));
            let completed = client
                .get(format!(
                    "/flow/complete?process_id={process_id}&outcome=success"
                ))
                .dispatch()
                .await;
            assert_eq!(
                completed.headers().get_one("Location"),
                Some(
                    format!(
                        "http://keycloak/return?code=1&processId={process_id}&token={process_id}"
                    )
                    .as_str()
                )
            );
            let session: serde_json::Value = client
                .get(format!("/api/v3/mobile_interaction/{process_id}/token"))
                .dispatch()
                .await
                .into_json()
                .await
                .unwrap();
            let session_token = session["token"].as_str().unwrap().to_string();
            client
                .get(format!(
                    "/api/v3/mobile_interaction/v2/{session_token}/results_with_image_names"
                ))
                .dispatch()
                .await
                .into_json::<serde_json::Value>()
                .await
                .unwrap()
        }
    };

    // Keycloak's Scanovate rules read these OCR fields, so the selected
    // synthetic voter must reach them unchanged.
    let result = results("XX", "synthetic-process").await;
    assert_eq!(result["data"]["success"], true);
    let ocr = &result["data"]["resultsList"][0];
    assert_eq!(ocr["process"], "ocr");
    assert_eq!(ocr["firstName"], "Ada");
    assert_eq!(ocr["lastName"], "Lovelace");
    assert_eq!(ocr["idNumber"], "SYN-17");
    assert_eq!(ocr["documentNumber"], "SYN-17");
    assert_eq!(ocr["dob"], "29.02.2000");
    assert_eq!(ocr["dateOfBirth"], "2000-02-29T00:00:00.000+0000");
    assert_eq!(ocr["issuingCountry"]["alpha3"], "PHL");

    // Without a voter for the country, the fixed voter echoes the typed id.
    let fallback = results("missing", "fallback-process").await;
    assert_eq!(fallback["data"]["resultsList"][0]["firstName"], "JUAN");
    assert_eq!(fallback["data"]["resultsList"][0]["idNumber"], "TYPED-1");

    let unknown = client
        .get("/api/v3/mobile_interaction/unknown/token")
        .dispatch()
        .await;
    assert_eq!(unknown.status(), rocket::http::Status::NotFound);
    // Reload replaces the prior collection rather than appending new UUID rows.
    assert_eq!(load_users("synthetic.csv").unwrap(), 1);
    assert_eq!(get_users_from_db().unwrap().len(), 1);
}

#[test]
fn malformed_database_rows_are_errors_not_an_apparently_empty_country() {
    let _lock = DATABASE.lock().unwrap_or_else(|error| error.into_inner());
    let (_temp, _cwd) = database();
    csv();
    load_users("synthetic.csv").unwrap();
    assert!(random_user_by_country("XX").unwrap().is_some());
    let conn = rusqlite::Connection::open("voters.db").unwrap();
    conn.execute("UPDATE voters SET first_name = NULL", [])
        .unwrap();
    assert!(get_users_from_db().is_err());
    let error = random_user_by_country("XX").unwrap_err();
    assert!(
        error.to_string().contains("Invalid column type Null"),
        "{error}"
    );
}

#[test]
fn invalid_dates_remain_literal_and_missing_csv_reports_the_io_context() {
    let _lock = DATABASE.lock().unwrap_or_else(|error| error.into_inner());
    let (_temp, _cwd) = database();
    csv();
    let input = std::fs::read_to_string("synthetic.csv")
        .unwrap()
        .replace("2000-02-29", "2001-02-29");
    std::fs::write("synthetic.csv", input).unwrap();
    load_users("synthetic.csv").unwrap();
    assert_eq!(get_users_from_db().unwrap()[0].date_of_birth, "2001-02-29");
    assert_eq!(
        load_users("missing.csv").unwrap_err().to_string(),
        "Error opening the CSV file"
    );
    assert_eq!(get_users_from_db().unwrap().len(), 1);
}
