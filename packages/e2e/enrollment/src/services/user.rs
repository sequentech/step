use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use rusqlite::{params, Connection};

use crate::types::user::User;

pub fn load_users() -> Result<(), anyhow::Error> {
    // 1. Define the CSV path
    let csv_path = "./voters.csv";

    // 3. Open the CSV with headers
    let mut rdr = ReaderBuilder::new()
        .has_headers(true) // if you have a header row in the CSV
        .from_path(csv_path)
        .context("Error opening the CSV file")?;

    // 4. Connect to SQLite
    let conn = Connection::open("voters.db")
        .context("Failed to open or create 'voters.db'")
        .context("Error creating sqlite connection")?;

    // 5. Create table if not exists (all columns as TEXT for simplicity)
    //    Adjust columns/types as needed for your data (BOOLEAN, INTEGER, etc.).
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS voters (
            id TEXT PRIMARY KEY,
            email TEXT,
            email_verified INTEGER,
            enabled INTEGER,
            first_name TEXT,
            last_name TEXT,
            username TEXT,
            area_name TEXT,
            middleName TEXT,
            emailAndOrMobile TEXT,
            mobile_number TEXT,
            dateOfBirth TEXT,
            otp_method TEXT,
            embassy TEXT,
            country TEXT,
            id_card_number_validated TEXT,
            authorized_election_ids TEXT,
            id_card_number TEXT,
            sex TEXT,
            landBasedOrSeafarer TEXT,
            clusteredPrecinct TEXT,
            overseasReferences TEXT,
            id_card_type TEXT,
            termsOfService TEXT
        );
        "#
    )
    .context("Failed to create 'voters' table")?;

    // 6. Insert records
    for record_result in rdr.records() {
        let record = 
        record_result
        .context("Failed to read record from CSV")
        .context("Failed to read record from CSV")?;

        // Extract each column by index, in the same order as your CSV
        let id                        = record.get(0).unwrap_or_default().trim().to_string();
        let email                     = record.get(1).unwrap_or_default().trim().to_string();
        let email_verified_str = record.get(2).unwrap_or_default().trim();
        let email_verified = if email_verified_str.eq_ignore_ascii_case("true") {
            1
        } else {
            0
        };
        let enabled_str = record.get(3).unwrap_or_default().trim();
        let enabled = if enabled_str.eq_ignore_ascii_case("true") {
            1
        } else {
            0
        };
        let first_name                = record.get(4).unwrap_or_default().trim().to_string();
        let last_name                 = record.get(5).unwrap_or_default().trim().to_string();
        let username                  = record.get(6).unwrap_or_default().trim().to_string();
        let area_name                 = record.get(7).unwrap_or_default().trim().to_string();
        let middleName                = record.get(8).unwrap_or_default().trim().to_string();
        let emailAndOrMobile          = record.get(9).unwrap_or_default().trim().to_string();
        let mobile_number             = record.get(10).unwrap_or_default().trim().to_string();
        let dateOfBirth               = record.get(11).unwrap_or_default().trim().to_string();
        let otp_method                = record.get(12).unwrap_or_default().trim().to_string();
        let embassy                   = record.get(13).unwrap_or_default().trim().to_string();
        let country                   = record.get(14).unwrap_or_default().trim().to_string();
        let id_card_number_validated  = record.get(15).unwrap_or_default().trim().to_string();
        let authorized_election_ids   = record.get(16).unwrap_or_default().trim().to_string();
        let id_card_number            = record.get(17).unwrap_or_default().trim().to_string();
        let sex                       = record.get(18).unwrap_or_default().trim().to_string();
        let landBasedOrSeafarer       = record.get(19).unwrap_or_default().trim().to_string();
        let clusteredPrecinct         = record.get(20).unwrap_or_default().trim().to_string();
        let overseasReferences        = record.get(21).unwrap_or_default().trim().to_string();
        let id_card_type              = record.get(22).unwrap_or_default().trim().to_string();
        let termsOfService            = record.get(23).unwrap_or_default().trim().to_string();

        conn.execute(
            r#"
            INSERT OR IGNORE INTO voters (
                id, email, email_verified, enabled, first_name, last_name, username,
                area_name, middleName, emailAndOrMobile, mobile_number, dateOfBirth,
                otp_method, embassy, country, id_card_number_validated,
                authorized_election_ids, id_card_number, sex, landBasedOrSeafarer,
                clusteredPrecinct, overseasReferences, id_card_type, termsOfService
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24
            )
            "#,
            params![
                id, email, email_verified, enabled, first_name, last_name, username,
                area_name, middleName, emailAndOrMobile, mobile_number, dateOfBirth,
                otp_method, embassy, country, id_card_number_validated,
                authorized_election_ids, id_card_number, sex, landBasedOrSeafarer,
                clusteredPrecinct, overseasReferences, id_card_type, termsOfService
            ],
        )
        .with_context(|| format!("Failed to insert row for id '{}'", id))
        .context(format!("Failed to insert row for id '{}'", id))?;
    }

    // On success
    Ok(())
}

pub fn get_users_from_db() -> anyhow::Result<Vec<User>> {
    let conn = rusqlite::Connection::open("voters.db")?;

    let mut stmt = conn.prepare(
        r#"
        SELECT 
            id, email, email_verified, enabled, first_name, last_name, username,
            area_name, middleName, mobile_number,
            otp_method, embassy, country, id_card_number, id_card_type
        FROM voters
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get::<_, String>(0)?,
            email: row.get::<_, String>(1)?,
            email_verified: row.get::<_, i64>(2)? != 0,
            enabled: row.get::<_, i64>(3)? != 0,
            first_name: row.get::<_, String>(4)?,
            last_name: row.get::<_, String>(5)?,
            username: row.get::<_, String>(6)?,
            area_name: row.get::<_, String>(7)?,
            middle_name: row.get::<_, String>(8)?,
            mobile_number: row.get::<_, String>(9)?,
            otp_method: row.get::<_, String>(10)?,
            embassy: row.get::<_, String>(11)?,
            country: row.get::<_, String>(12)?,
            id_card_number: row.get::<_, String>(13)?,
            id_card_type: row.get::<_, String>(14)?,
        })
    })?;

    let mut users = Vec::new();
    for user_res in rows {
        users.push(user_res?);
    }

    Ok(users)
}