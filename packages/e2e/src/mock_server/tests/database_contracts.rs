// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[macro_use]
extern crate rocket;
#[path = "../src/routes/inetum.rs"]
mod inetum;
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
    let client = rocket::local::asynchronous::Client::tracked(rocket::build().mount(
        "/",
        routes![
            inetum::transaction_new,
            inetum::transaction_status_simple,
            inetum::transaction_results
        ],
    ))
    .await
    .unwrap();
    let created: serde_json::Value = client
        .post("/transaction/new")
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    let user = created["response"]["user_id"].as_str().unwrap();
    let token = created["response"]["token_dob"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(user).is_ok());
    assert!(uuid::Uuid::parse_str(token).is_ok());
    assert_ne!(user, token);
    let status: serde_json::Value = client
        .get("/status")
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    assert_eq!(
        status,
        serde_json::json!({"code":0,"response":{"idStatus":"verificationOK"}})
    );
    let result: serde_json::Value = client
        .get("/results?country=XX")
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    assert_eq!(
        result["response"]["ocr"],
        serde_json::json!({"issuing_state_code":"PHL", "given_names":"Ada", "middle_name":"M", "surname":"Lovelace", "personal_number":"SYN-17", "date_of_birth":"29/02/2000"})
    );
    let absent = client.get("/results?country=missing").dispatch().await;
    assert_eq!(absent.status(), rocket::http::Status::InternalServerError);
    assert_eq!(absent.into_string().await.unwrap(), "User not found");
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
