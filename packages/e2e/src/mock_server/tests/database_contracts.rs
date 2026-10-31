// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[path = "../src/services/mod.rs"]
mod services;
#[path = "../src/types/mod.rs"]
mod types;

use services::user::{get_users_from_db, load_users};
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

#[test]
fn csv_mapping_preserves_the_synthetic_identity() {
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
    assert_eq!(get_users_from_db().unwrap().len(), 1);
    let conn = rusqlite::Connection::open("voters.db").unwrap();
    conn.execute("UPDATE voters SET first_name = NULL", [])
        .unwrap();
    let error = get_users_from_db().unwrap_err();
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
